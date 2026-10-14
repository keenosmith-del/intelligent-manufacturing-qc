use anyhow::{bail, Result};
use std::{env, path::PathBuf};

pub struct Config {
    pub port: u16,
    pub bind_address: String,
    pub project: String,
    pub emulator: Option<String>,
    pub storage_provider: String,
    pub local_storage: PathBuf,
    pub artifact_root: PathBuf,
    pub bucket: Option<String>,
    pub cors_origin: String,
    pub admin_token: String,
    pub allow_synthetic: bool,
    pub min_f1: f64,
    pub min_recall: f64,
    pub max_regression: f64,
}
fn setting(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.into())
}
impl Config {
    pub fn from_env() -> Result<Self> {
        let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
        if admin_token.len() < 16 {
            bail!("ADMIN_TOKEN must contain at least 16 characters");
        }
        let config = Self {
            port: setting("PORT", "5050").parse()?,
            bind_address: setting("BIND_ADDRESS", "127.0.0.1"),
            project: setting("FIRESTORE_PROJECT_ID", "manufacturing-qc-local"),
            emulator: env::var("FIRESTORE_EMULATOR_HOST")
                .ok()
                .filter(|s| !s.is_empty()),
            storage_provider: setting("STORAGE_PROVIDER", "local"),
            local_storage: setting("LOCAL_STORAGE_PATH", "../.storage").into(),
            artifact_root: setting("MODEL_ARTIFACT_PATH", "../artifacts").into(),
            bucket: env::var("GCS_BUCKET").ok(),
            cors_origin: setting("CORS_ORIGIN", "http://localhost:5173"),
            admin_token,
            allow_synthetic: setting("ALLOW_SYNTHETIC", "false") == "true",
            min_f1: setting("MIN_MACRO_F1", "0.5").parse()?,
            min_recall: setting("MIN_CLASS_RECALL", "0.2").parse()?,
            max_regression: setting("MAX_F1_REGRESSION", "0.03").parse()?,
        };
        for v in [config.min_f1, config.min_recall, config.max_regression] {
            if !v.is_finite() || !(0.0..=1.0).contains(&v) {
                bail!("invalid gate threshold");
            }
        }
        if config.emulator.is_none() && config.allow_synthetic {
            bail!("synthetic models are restricted to emulator mode");
        }
        Ok(config)
    }
}
