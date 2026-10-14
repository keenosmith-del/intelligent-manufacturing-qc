use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CLASSES: [&str; 6] = [
    "crazing",
    "inclusion",
    "patches",
    "pitted_surface",
    "rolled_in_scale",
    "scratches",
];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preprocessing {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub layout: String,
    pub resize: String,
    pub grayscale: String,
    pub normalization: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub schema_version: u32,
    pub version: String,
    pub classes: Vec<String>,
    pub preprocessing: Preprocessing,
    pub dataset_version: String,
    pub test_split_hash: String,
    pub provenance: Value,
    pub framework: String,
    pub architecture: String,
    pub training_config: Value,
    pub started_at: String,
    pub finished_at: String,
    pub history: Value,
    pub evaluation: Value,
    pub parity: Value,
    pub sha256: String,
    pub artifact_bytes: u64,
    pub reference_distribution: Vec<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelRecord {
    pub metadata: Metadata,
    pub status: String,
    pub gates: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prediction {
    pub label: String,
    pub confidence: f32,
    pub probabilities: Vec<f32>,
    pub model_version: String,
    pub dataset_version: String,
    pub synthetic: bool,
    pub inference_ms: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub inspection_id: String,
    pub label: String,
    pub note: String,
    pub reviewer: String,
    pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inspection {
    pub id: String,
    pub image_key: String,
    pub media_type: String,
    pub prediction: Prediction,
    pub created_at: String,
    pub status: String,
    pub review_status: String,
    pub review: Option<Review>,
    pub total_ms: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveModel {
    pub version: String,
    pub previous: Option<String>,
    pub activated_at: String,
}
pub fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && s != "."
        && s != ".."
}
pub fn timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiers() {
        for value in ["..", ".", "", "foo/bar", "%2fsecret", "a?b"] {
            assert!(!valid_id(value));
        }
        assert!(valid_id("v1-smoke.2"));
    }
}
