use crate::{
    error::{ApiError, ApiResult},
    models::*,
    monitoring, registry, AppState,
};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Instant,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

type Shared = State<Arc<AppState>>;
pub fn router(state: Arc<AppState>) -> Router {
    let origin = state
        .config
        .cors_origin
        .parse::<axum::http::HeaderValue>()
        .expect("valid CORS origin");
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/classes", get(|| async { Json(CLASSES) }))
        .route(
            "/api/inspections",
            get(list_inspections).post(create_inspection),
        )
        .route("/api/inspections/{id}", get(detail))
        .route("/api/inspections/{id}/image", get(image))
        .route("/api/inspections/{id}/reviews", get(reviews).post(review))
        .route("/api/overview", get(overview))
        .route("/api/models", get(models).post(register_model))
        .route("/api/models/{version}/validate", post(validate_model))
        .route("/api/models/{version}/activate", post(activate_model))
        .route("/api/models/rollback", post(rollback))
        .route("/api/training-runs", get(training_runs))
        .route("/api/releases", get(releases))
        .route("/api/monitoring", get(monitor))
        .route("/api/monitoring/snapshots", get(snapshots).post(snapshot))
        .layer(DefaultBodyLimit::max(11 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
fn admin(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    let expected = format!("Bearer {}", state.config.admin_token);
    let actual = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    // Constant-time comparison for equal-length tokens.
    let equal = actual.len() == expected.len()
        && actual
            .bytes()
            .zip(expected.bytes())
            .fold(0u8, |a, (x, y)| a | (x ^ y))
            == 0;
    if !equal {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "Operator token required".into(),
        ));
    }
    Ok(())
}
fn id(value: &str) -> ApiResult<()> {
    if !valid_id(value) {
        Err(ApiError::bad("Invalid identifier"))
    } else {
        Ok(())
    }
}
async fn health() -> Json<Value> {
    Json(json!({"status":"alive"}))
}
async fn ready(State(s): Shared) -> ApiResult<Json<Value>> {
    s.db.ping().await?;
    let engine = s
        .engine
        .read()
        .await
        .clone()
        .ok_or_else(|| ApiError::unavailable("No active model"))?;
    let (version, load_ms) = tokio::task::spawn_blocking(move || {
        let mut e = engine.blocking_lock();
        let size = (e.metadata.preprocessing.width * e.metadata.preprocessing.height) as usize;
        e.run(vec![0.0; size])?;
        Ok::<_, anyhow::Error>((e.metadata.version.clone(), e.load_ms))
    })
    .await
    .map_err(|_| ApiError::unavailable("Inference worker failed"))??;
    Ok(Json(
        json!({"status":"ready","model_version":version,"model_load_ms":load_ms}),
    ))
}
async fn create_inspection(
    State(s): Shared,
    mut multipart: Multipart,
) -> ApiResult<Json<Inspection>> {
    s.requests.fetch_add(1, Ordering::Relaxed);
    let result = process_inspection(&s, &mut multipart).await;
    if result.is_err() {
        s.failures.fetch_add(1, Ordering::Relaxed);
    }
    result.map(Json)
}
async fn process_inspection(s: &AppState, multipart: &mut Multipart) -> ApiResult<Inspection> {
    let start = Instant::now();
    let field = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad("Invalid multipart upload"))?
        .ok_or_else(|| ApiError::bad("Image field is required"))?;
    if field.name() != Some("image") {
        return Err(ApiError::bad("Expected field named image"));
    }
    let bytes = field
        .bytes()
        .await
        .map_err(|_| ApiError::bad("Upload exceeds limit or is malformed"))?
        .to_vec();
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(ApiError::bad("Maximum image size is 10 MiB"));
    }
    let engine = s.engine.read().await.clone().ok_or_else(|| {
        ApiError::unavailable("No active model; register and activate a validated model")
    })?;
    let image_bytes = bytes.clone();
    let (prediction, media) =
        tokio::task::spawn_blocking(move || engine.blocking_lock().predict(&image_bytes))
            .await
            .map_err(|_| ApiError::unavailable("Inference worker failed"))?
            .map_err(|error| {
                tracing::warn!(%error,"image inference failed");
                ApiError::bad("Image invalid or model inference failed")
            })?;
    let inspection_id = uuid::Uuid::new_v4().to_string();
    let key = format!(
        "images/{inspection_id}.{}",
        match media {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            _ => "bmp",
        }
    );
    s.storage.put(&key, &bytes, media).await?;
    let inspection = Inspection {
        id: inspection_id,
        image_key: key.clone(),
        media_type: media.into(),
        prediction,
        created_at: timestamp(),
        status: "completed".into(),
        review_status: "unreviewed".into(),
        review: None,
        total_ms: start.elapsed().as_secs_f64() * 1000.0,
    };
    if let Err(error) = s.db.set("inspections", &inspection.id, &inspection).await {
        // A timeout can occur after a successful commit. Never delete evidence blindly.
        match s.db.get::<Inspection>("inspections", &inspection.id).await {
            Ok(Some(persisted)) => return Ok(persisted),
            Ok(None) => {
                if let Err(cleanup) = s.storage.delete(&key).await {
                    tracing::error!(%cleanup, "orphan image cleanup failed");
                }
            }
            Err(reconciliation) => {
                tracing::error!(%reconciliation, "commit outcome unknown; image retained for reconciliation");
            }
        }
        return Err(error.into());
    }
    Ok(inspection)
}
#[derive(Deserialize, Default)]
struct Filters {
    search: Option<String>,
    category: Option<String>,
    review_status: Option<String>,
    max_confidence: Option<f32>,
    page: Option<usize>,
    limit: Option<usize>,
    sort: Option<String>,
}
async fn list_inspections(State(s): Shared, Query(f): Query<Filters>) -> ApiResult<Json<Value>> {
    if f.max_confidence
        .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
    {
        return Err(ApiError::bad("Invalid confidence filter"));
    }
    let mut rows = s.db.list::<Inspection>("inspections").await?;
    rows.retain(|r| {
        f.search
            .as_ref()
            .is_none_or(|q| r.id.contains(q) || r.prediction.label.contains(q))
            && f.category.as_ref().is_none_or(|q| &r.prediction.label == q)
            && f.review_status
                .as_ref()
                .is_none_or(|q| &r.review_status == q)
            && f.max_confidence
                .is_none_or(|max| r.prediction.confidence <= max)
    });
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    if f.sort.as_deref() == Some("oldest") {
        rows.reverse();
    }
    let page = f.page.unwrap_or(1).max(1);
    let limit = f.limit.unwrap_or(20).clamp(1, 100);
    let total = rows.len();
    let items: Vec<_> = rows
        .into_iter()
        .skip(page.saturating_sub(1).saturating_mul(limit))
        .take(limit)
        .collect();
    Ok(Json(
        json!({"items":items,"total":total,"page":page,"limit":limit}),
    ))
}
async fn detail(State(s): Shared, Path(value): Path<String>) -> ApiResult<Json<Inspection>> {
    id(&value)?;
    Ok(Json(
        s.db.get("inspections", &value)
            .await?
            .ok_or_else(ApiError::not_found)?,
    ))
}
async fn image(State(s): Shared, Path(value): Path<String>) -> ApiResult<impl IntoResponse> {
    id(&value)?;
    let row =
        s.db.get::<Inspection>("inspections", &value)
            .await?
            .ok_or_else(ApiError::not_found)?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, row.media_type),
            (
                axum::http::header::CACHE_CONTROL,
                "private, max-age=3600".into(),
            ),
            (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        s.storage.get(&row.image_key).await?,
    ))
}
#[derive(Deserialize)]
struct ReviewInput {
    label: String,
    note: String,
    reviewer: String,
}
async fn review(
    State(s): Shared,
    Path(value): Path<String>,
    headers: HeaderMap,
    Json(input): Json<ReviewInput>,
) -> ApiResult<Json<Inspection>> {
    admin(&s, &headers)?;
    id(&value)?;
    if !CLASSES.contains(&input.label.as_str())
        || input.note.len() > 2000
        || input.reviewer.trim().is_empty()
        || input.reviewer.len() > 100
    {
        return Err(ApiError::bad(
            "Valid class, reviewer (1..100 chars) and note (<=2000 chars) required",
        ));
    }
    // The V1 single-process lock avoids lost review updates and serializes audit commits.
    let _lock = s.release_lock.lock().await;
    let mut row =
        s.db.get::<Inspection>("inspections", &value)
            .await?
            .ok_or_else(ApiError::not_found)?;
    let review = Review {
        id: uuid::Uuid::new_v4().to_string(),
        inspection_id: value.clone(),
        label: input.label,
        note: input.note,
        reviewer: input.reviewer,
        created_at: timestamp(),
    };
    row.review = Some(review.clone());
    row.review_status = "reviewed".into();
    s.db.commit(vec![("inspections".into(),value,serde_json::to_value(&row).map_err(anyhow::Error::from)?),
        ("inspection_reviews".into(),review.id.clone(),serde_json::to_value(&review).map_err(anyhow::Error::from)?),
        ("audit_events".into(),uuid::Uuid::new_v4().to_string(),json!({"action":"review","inspection_id":row.id,"review_id":review.id,"at":timestamp()}))]).await?;
    Ok(Json(row))
}
async fn reviews(State(s): Shared, Path(value): Path<String>) -> ApiResult<Json<Vec<Review>>> {
    id(&value)?;
    let mut rows = s.db.list::<Review>("inspection_reviews").await?;
    rows.retain(|r| r.inspection_id == value);
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(Json(rows))
}
async fn models(State(s): Shared) -> ApiResult<Json<Value>> {
    Ok(Json(
        json!({"items":s.db.list::<ModelRecord>("model_versions").await?,"active":s.db.get::<ActiveModel>("system","active").await?}),
    ))
}
#[derive(Deserialize)]
struct RegisterInput {
    version: String,
}
async fn register_model(
    State(s): Shared,
    headers: HeaderMap,
    Json(input): Json<RegisterInput>,
) -> ApiResult<Json<ModelRecord>> {
    admin(&s, &headers)?;
    id(&input.version)?;
    Ok(Json(registry::register(&s, &input.version).await.map_err(
        |e| {
            tracing::warn!(%e,"registration rejected");
            ApiError::bad("Artifact invalid, missing or version already registered")
        },
    )?))
}
async fn validate_model(
    State(s): Shared,
    Path(version): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<ModelRecord>> {
    admin(&s, &headers)?;
    id(&version)?;
    Ok(Json(registry::validate(&s, &version).await?))
}
async fn activate_model(
    State(s): Shared,
    Path(version): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<ActiveModel>> {
    admin(&s, &headers)?;
    id(&version)?;
    Ok(Json(
        registry::activate(&s, &version, false).await.map_err(|e| {
            tracing::warn!(%e,"activation rejected");
            ApiError::bad("Activation rejected; inspect release gates and service logs")
        })?,
    ))
}
async fn rollback(State(s): Shared, headers: HeaderMap) -> ApiResult<Json<ActiveModel>> {
    admin(&s, &headers)?;
    let active =
        s.db.get::<ActiveModel>("system", "active")
            .await?
            .ok_or_else(|| ApiError::bad("No active model"))?;
    let previous = active
        .previous
        .ok_or_else(|| ApiError::bad("No previous model"))?;
    Ok(Json(registry::activate(&s, &previous, true).await?))
}
async fn training_runs(State(s): Shared) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(s.db.list("training_runs").await?))
}
async fn releases(State(s): Shared) -> ApiResult<Json<Vec<Value>>> {
    let mut rows = s.db.list::<Value>("model_releases").await?;
    rows.sort_by(|a, b| b["at"].as_str().cmp(&a["at"].as_str()));
    Ok(Json(rows))
}
async fn monitoring_value(s: &AppState) -> ApiResult<Value> {
    let mut rows = s.db.list::<Inspection>("inspections").await?;
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let active = s.db.get::<ActiveModel>("system", "active").await?;
    let record = if let Some(a) = &active {
        s.db.get::<ModelRecord>("model_versions", &a.version)
            .await?
    } else {
        None
    };
    let mut metrics = monitoring::summarize(&rows, None);
    // Drift compares only the active model's observations to that model's reference.
    let active_rows: Vec<_> = rows
        .iter()
        .filter(|r| {
            active
                .as_ref()
                .is_some_and(|a| a.version == r.prediction.model_version)
        })
        .cloned()
        .collect();
    let current = monitoring::summarize(
        &active_rows,
        record
            .as_ref()
            .map(|r| r.metadata.reference_distribution.as_slice()),
    );
    metrics["active_model_metrics"] = current;
    metrics["active"] = serde_json::to_value(active).map_err(anyhow::Error::from)?;
    metrics["active_evaluation"] = record
        .as_ref()
        .map(|r| r.metadata.evaluation.clone())
        .unwrap_or(Value::Null);
    metrics["synthetic"] = record
        .map(|r| r.metadata.provenance["synthetic"].clone())
        .unwrap_or(Value::Null);
    let requests = s.requests.load(Ordering::Relaxed);
    let failures = s.failures.load(Ordering::Relaxed);
    metrics["process_telemetry"] = json!({"requests":requests,"failures":failures,"error_rate":if requests==0{0.0}else{failures as f64/requests as f64},"scope":"since API startup"});
    metrics["recent"] = serde_json::to_value(rows.into_iter().take(6).collect::<Vec<_>>())
        .map_err(anyhow::Error::from)?;
    metrics["model_loaded"] = json!(s.engine.read().await.is_some());
    Ok(metrics)
}
async fn overview(State(s): Shared) -> ApiResult<Json<Value>> {
    Ok(Json(monitoring_value(&s).await?))
}
async fn monitor(State(s): Shared) -> ApiResult<Json<Value>> {
    Ok(Json(monitoring_value(&s).await?))
}
async fn snapshots(State(s): Shared) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(s.db.list("monitoring_snapshots").await?))
}
async fn snapshot(State(s): Shared, headers: HeaderMap) -> ApiResult<Json<Value>> {
    admin(&s, &headers)?;
    let value = json!({"id":uuid::Uuid::new_v4().to_string(),"created_at":timestamp(),"metrics":monitoring_value(&s).await?});
    s.db.set(
        "monitoring_snapshots",
        value["id"].as_str().unwrap(),
        &value,
    )
    .await?;
    Ok(Json(value))
}
