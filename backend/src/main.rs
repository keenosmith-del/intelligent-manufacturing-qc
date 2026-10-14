mod config;
mod error;
mod firestore;
mod inference;
mod models;
mod monitoring;
mod registry;
mod routes;
mod storage;

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct AppState {
    config: config::Config,
    db: firestore::Firestore,
    storage: storage::Storage,
    engine: RwLock<Option<Arc<Mutex<inference::Engine>>>>,
    release_lock: Mutex<()>,
    pub requests: std::sync::atomic::AtomicU64,
    pub failures: std::sync::atomic::AtomicU64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();
    let config = config::Config::from_env()?;
    // An unavailable runtime must stop startup; missing models keep readiness false.
    ort::init().with_name("manufacturing-qc").commit();
    let _runtime_check = ort::session::Session::builder()?;
    let db = firestore::Firestore::new(&config);
    let storage = storage::Storage::new(&config)?;
    let address = format!("{}:{}", config.bind_address, config.port);
    let state = Arc::new(AppState {
        config,
        db,
        storage,
        engine: RwLock::new(None),
        release_lock: Mutex::new(()),
        requests: 0.into(),
        failures: 0.into(),
    });
    if let Err(error) = registry::restore(&state).await {
        tracing::warn!(%error, "active model restore failed; readiness is false");
    }
    let router = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(%address, "API listening");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
