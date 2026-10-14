//! One-process release coordinator: prepare executable session, commit pointer, then swap.
use crate::{
    inference::Engine,
    models::{timestamp, ActiveModel, Metadata, ModelRecord, CLASSES},
    AppState,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn load_registered(state: &AppState, record: &ModelRecord) -> Result<Engine> {
    let bytes = state
        .storage
        .get(&format!("models/{}/model.onnx", record.metadata.version))
        .await?;
    let metadata = record.metadata.clone();
    tokio::task::spawn_blocking(move || Engine::load(metadata, &bytes)).await?
}
pub async fn restore(state: &AppState) -> Result<()> {
    if let Some(active) = state.db.get::<ActiveModel>("system", "active").await? {
        let record = state
            .db
            .get::<ModelRecord>("model_versions", &active.version)
            .await?
            .context("active model missing")?;
        if record.status != "validated" {
            bail!("active model is not validated");
        }
        let engine = load_registered(state, &record).await?;
        *state.engine.write().await = Some(Arc::new(Mutex::new(engine)));
    }
    Ok(())
}
pub async fn register(state: &AppState, version: &str) -> Result<ModelRecord> {
    let _lock = state.release_lock.lock().await;
    if state
        .db
        .get::<ModelRecord>("model_versions", version)
        .await?
        .is_some()
    {
        bail!("model version already exists");
    }
    let folder = state.config.artifact_root.join(version);
    // Local import root is operator-configured. Canonical containment rejects symlink escape.
    let root = tokio::fs::canonicalize(&state.config.artifact_root).await?;
    let folder = tokio::fs::canonicalize(folder).await?;
    if !folder.starts_with(root) {
        bail!("artifact path escapes configured root");
    }
    let metadata: Metadata =
        serde_json::from_slice(&tokio::fs::read(folder.join("metadata.json")).await?)?;
    if metadata.version != version {
        bail!("version mismatch");
    }
    let bytes = tokio::fs::read(folder.join("model.onnx")).await?;
    if bytes.len() > 100 * 1024 * 1024 {
        bail!("model exceeds V1 artifact size limit");
    }
    if metadata.artifact_bytes != bytes.len() as u64 {
        bail!("artifact size mismatch");
    }
    let m = metadata.clone();
    let model_bytes = bytes.clone();
    tokio::task::spawn_blocking(move || Engine::load(m, &model_bytes)).await??;
    // Copy package documents to object storage, never model binaries into Firestore.
    state
        .storage
        .put(
            &format!("models/{version}/model.onnx"),
            &bytes,
            "application/octet-stream",
        )
        .await?;
    for file in [
        "metadata.json",
        "evaluation.json",
        "classes.json",
        "preprocessing.json",
        "parity.json",
        "manifest.json",
        "training-config.json",
    ] {
        let data = tokio::fs::read(folder.join(file)).await?;
        state
            .storage
            .put(
                &format!("models/{version}/{file}"),
                &data,
                "application/json",
            )
            .await?;
    }
    let record = ModelRecord {
        metadata: metadata.clone(),
        status: "candidate".into(),
        gates: None,
    };
    state.db.commit(vec![
        ("model_versions".into(), version.into(), serde_json::to_value(&record)?),
        ("model_evaluations".into(), version.into(), metadata.evaluation.clone()),
        ("training_runs".into(), version.into(), json!({"version":version,"config":metadata.training_config,
            "history":metadata.history,"started_at":metadata.started_at,"finished_at":metadata.finished_at,
            "dataset_version":metadata.dataset_version})),
        ("audit_events".into(), uuid::Uuid::new_v4().to_string(), json!({"action":"register", "version":version,"at":timestamp()}))
    ]).await?;
    Ok(record)
}
pub fn gates(
    candidate: &ModelRecord,
    baseline: Option<&ModelRecord>,
    config: &crate::config::Config,
) -> Value {
    let m = &candidate.metadata;
    let mut reasons = Vec::new();
    if m.provenance["synthetic"] == true && !config.allow_synthetic {
        reasons.push("Synthetic model requires explicitly enabled local emulator mode".to_owned());
    }
    if m.evaluation["macro_f1"].as_f64().unwrap_or(0.0) < config.min_f1 {
        reasons.push("Macro-F1 below threshold".into());
    }
    for c in CLASSES {
        if m.evaluation["per_class"][c]["recall"]
            .as_f64()
            .unwrap_or(0.0)
            < config.min_recall
        {
            reasons.push(format!("Recall below threshold: {c}"));
        }
    }
    if let Some(base) = baseline {
        if m.dataset_version != base.metadata.dataset_version
            || m.test_split_hash != base.metadata.test_split_hash
        {
            reasons
                .push("Incompatible dataset or test split: regression comparison refused".into());
        } else if m.evaluation["macro_f1"].as_f64().unwrap_or(0.0) + config.max_regression
            < base.metadata.evaluation["macro_f1"].as_f64().unwrap_or(0.0)
        {
            reasons.push("Macro-F1 regression exceeds threshold".into());
        }
    }
    json!({"passed":reasons.is_empty(), "reasons":reasons, "checked_at":timestamp(),
        "thresholds":{"min_macro_f1":config.min_f1,"min_class_recall":config.min_recall,
            "max_f1_regression":config.max_regression},
        "baseline":baseline.map(|b| b.metadata.version.clone())})
}
pub async fn validate(state: &AppState, version: &str) -> Result<ModelRecord> {
    let _lock = state.release_lock.lock().await;
    validate_locked(state, version).await
}
async fn validate_locked(state: &AppState, version: &str) -> Result<ModelRecord> {
    let mut record = state
        .db
        .get::<ModelRecord>("model_versions", version)
        .await?
        .context("model missing")?;
    let active = state.db.get::<ActiveModel>("system", "active").await?;
    let baseline = if let Some(active) = active {
        state
            .db
            .get::<ModelRecord>("model_versions", &active.version)
            .await?
    } else {
        None
    };
    // Runtime compatibility is part of the gate, alongside independently stored evaluation.
    let _engine = load_registered(state, &record).await?;
    let result = gates(&record, baseline.as_ref(), &state.config);
    record.status = if result["passed"] == true {
        "validated"
    } else {
        "rejected"
    }
    .into();
    record.gates = Some(result.clone());
    state
        .db
        .commit(vec![
            (
                "model_versions".into(),
                version.into(),
                serde_json::to_value(&record)?,
            ),
            (
                "model_releases".into(),
                uuid::Uuid::new_v4().to_string(),
                json!({"action":"validation","version":version,"at":timestamp(),"gates":result}),
            ),
        ])
        .await?;
    Ok(record)
}
pub async fn activate(state: &AppState, version: &str, rollback: bool) -> Result<ActiveModel> {
    let _lock = state.release_lock.lock().await;
    let old = state.db.get::<ActiveModel>("system", "active").await?;
    if old.as_ref().is_some_and(|a| a.version == version) {
        bail!("model already active");
    }
    let record = if rollback {
        if old.as_ref().and_then(|a| a.previous.as_deref()) != Some(version) {
            bail!("rollback target is not the previous model");
        }
        let record = state
            .db
            .get::<ModelRecord>("model_versions", version)
            .await?
            .context("rollback model missing")?;
        if record.status != "validated" {
            bail!("rollback model is not validated");
        }
        record
    } else {
        validate_locked(state, version).await?
    };
    if record.status != "validated" {
        bail!("release gates rejected candidate");
    }
    let engine = load_registered(state, &record).await?;
    let active = ActiveModel {
        version: version.into(),
        previous: old.map(|a| a.version),
        activated_at: timestamp(),
    };
    // Keep new requests off the session until the pointer and audit are committed.
    let mut slot = state.engine.write().await;
    let release_id = uuid::Uuid::new_v4().to_string();
    state
        .db
        .commit(vec![
            (
                "system".into(),
                "active".into(),
                serde_json::to_value(&active)?,
            ),
            (
                "model_releases".into(),
                release_id.clone(),
                json!({"id":release_id,"action":if rollback {"rollback"} else {"activation"},
            "version":version,"previous":active.previous,"at":active.activated_at}),
            ),
            (
                "audit_events".into(),
                uuid::Uuid::new_v4().to_string(),
                json!({"action":"release","version":version,"at":timestamp()}),
            ),
        ])
        .await?;
    *slot = Some(Arc::new(Mutex::new(engine)));
    Ok(active)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> crate::config::Config {
        crate::config::Config {
            port: 5050,
            bind_address: "127.0.0.1".into(),
            project: "test".into(),
            emulator: Some("127.0.0.1:8080".into()),
            storage_provider: "local".into(),
            local_storage: "unused".into(),
            artifact_root: "unused".into(),
            bucket: None,
            cors_origin: "http://localhost:5173".into(),
            admin_token: "test-operator-token".into(),
            allow_synthetic: true,
            min_f1: 0.5,
            min_recall: 0.2,
            max_regression: 0.03,
        }
    }
    fn fixture() -> ModelRecord {
        let m: Metadata =
            serde_json::from_str(include_str!("../../tests/fixtures/trained/metadata.json"))
                .unwrap();
        ModelRecord {
            metadata: m,
            status: "candidate".into(),
            gates: None,
        }
    }
    #[test]
    fn gates_reject_regression_and_incompatible_splits() {
        let base = fixture();
        let mut candidate = fixture();
        assert_eq!(gates(&candidate, Some(&base), &config())["passed"], true);
        candidate.metadata.evaluation["macro_f1"] = json!(0.9);
        assert_eq!(gates(&candidate, Some(&base), &config())["passed"], false);
        candidate = fixture();
        candidate.metadata.test_split_hash = "f".repeat(64);
        assert_eq!(gates(&candidate, Some(&base), &config())["passed"], false);
    }
    #[test]
    fn synthetic_cloud_and_class_recall_gate() {
        let mut c = config();
        c.allow_synthetic = false;
        assert_eq!(gates(&fixture(), None, &c)["passed"], false);
        let mut candidate = fixture();
        candidate.metadata.evaluation["per_class"]["crazing"]["recall"] = json!(0.0);
        assert_eq!(gates(&candidate, None, &config())["passed"], false);
    }
}
