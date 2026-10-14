use crate::models::{Metadata, Prediction, Preprocessing, CLASSES};
use anyhow::{bail, Context, Result};
use image::{ImageFormat, ImageReader};
use ort::{session::Session, value::Tensor};
use std::{io::Cursor, time::Instant};

pub struct Engine {
    pub metadata: Metadata,
    session: Session,
    pub load_ms: f64,
}

pub fn preprocess(bytes: &[u8], prep: &Preprocessing) -> Result<(Vec<f32>, &'static str)> {
    if bytes.is_empty() || bytes.len() > 10 * 1024 * 1024 {
        bail!("image must be 1 byte to 10 MiB");
    }
    let format = image::guess_format(bytes)?;
    let media = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Bmp => "image/bmp",
        _ => bail!("only PNG, JPEG and BMP are supported"),
    };
    let reader = ImageReader::with_format(Cursor::new(bytes), format);
    let (width, height) = reader.into_dimensions()?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 16_000_000 {
        bail!("image exceeds 16 megapixels");
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let rgb = reader.decode()?.to_rgb8();
    let mut data = Vec::with_capacity((prep.width * prep.height) as usize);
    for y in 0..prep.height {
        for x in 0..prep.width {
            let p = rgb.get_pixel(x * width / prep.width, y * height / prep.height);
            let gray =
                (299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2]) + 500)
                    / 1000;
            data.push(gray as f32 / 255.0);
        }
    }
    Ok((data, media))
}
impl Engine {
    pub fn load(metadata: Metadata, bytes: &[u8]) -> Result<Self> {
        validate_metadata(&metadata)?;
        let start = Instant::now();
        use sha2::Digest;
        if hex::encode(sha2::Sha256::digest(bytes)) != metadata.sha256 {
            bail!("model checksum mismatch");
        }
        let session = Session::builder()?
            .with_intra_threads(1)?
            .commit_from_memory(bytes)?;
        let mut engine = Self {
            metadata,
            session,
            load_ms: start.elapsed().as_secs_f64() * 1000.0,
        };
        let prep = &engine.metadata.preprocessing;
        // Executable shape/output check before a session can become active.
        engine.run(vec![0.0; (prep.width * prep.height) as usize])?;
        Ok(engine)
    }
    pub fn run(&mut self, pixels: Vec<f32>) -> Result<Vec<f32>> {
        let prep = &self.metadata.preprocessing;
        if pixels.len() != (prep.width * prep.height) as usize {
            bail!("pixel tensor shape mismatch");
        }
        let input =
            Tensor::from_array(([1, prep.height as usize, prep.width as usize, 1], pixels))?;
        let outputs = self.session.run(ort::inputs![input])?;
        let (shape, values) = outputs[0].try_extract_tensor::<f32>()?;
        if shape.as_ref() != [1, 6] || values.len() != 6 {
            bail!("expected [1,6] output");
        }
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || (values.iter().sum::<f32>() - 1.0).abs() > 1e-4
        {
            bail!("invalid probability output");
        }
        Ok(values.to_vec())
    }
    pub fn predict(&mut self, bytes: &[u8]) -> Result<(Prediction, &'static str)> {
        let (pixels, media) = preprocess(bytes, &self.metadata.preprocessing)?;
        let start = Instant::now();
        let values = self.run(pixels)?;
        let index = values
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .context("empty output")?
            .0;
        Ok((
            Prediction {
                label: CLASSES[index].into(),
                confidence: values[index],
                probabilities: values,
                model_version: self.metadata.version.clone(),
                dataset_version: self.metadata.dataset_version.clone(),
                synthetic: self.metadata.provenance["synthetic"]
                    .as_bool()
                    .unwrap_or(true),
                inference_ms: start.elapsed().as_secs_f64() * 1000.0,
            },
            media,
        ))
    }
}
pub fn validate_metadata(m: &Metadata) -> Result<()> {
    let p = &m.preprocessing;
    if m.schema_version != 1
        || m.classes != CLASSES
        || p.channels != 1
        || p.layout != "NHWC"
        || p.resize != "integer_nearest"
        || p.grayscale != "integer_601"
        || p.normalization != "divide_255"
        || !(8..=512).contains(&p.width)
        || !(8..=512).contains(&p.height)
    {
        bail!("unsupported model contract");
    }
    if m.provenance["synthetic"].as_bool().is_none() {
        bail!("synthetic provenance must be boolean");
    }
    if m.parity["passed"] != true
        || m.parity["same_classes"] != true
        || m.parity["max_absolute_error"]
            .as_f64()
            .unwrap_or(f64::INFINITY)
            > 1e-5
    {
        bail!("parity verification missing or failed");
    }
    if m.dataset_version.len() != 64 || m.test_split_hash.len() != 64 || m.sha256.len() != 64 {
        bail!("dataset, split and artifact hashes are required");
    }
    for key in ["accuracy", "macro_f1", "weighted_f1"] {
        let v = m.evaluation[key].as_f64().context("missing metric")?;
        if !(0.0..=1.0).contains(&v) {
            bail!("metric out of range");
        }
    }
    for c in CLASSES {
        let v = m.evaluation["per_class"][c]["recall"]
            .as_f64()
            .context("missing recall")?;
        if !(0.0..=1.0).contains(&v) {
            bail!("recall out of range");
        }
    }
    if m.reference_distribution.len() != 6
        || m.reference_distribution
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        || (m.reference_distribution.iter().sum::<f64>() - 1.0).abs() > 1e-5
    {
        bail!("invalid reference distribution");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_image_rejected() {
        let p = Preprocessing {
            width: 8,
            height: 8,
            channels: 1,
            layout: "NHWC".into(),
            resize: "integer_nearest".into(),
            grayscale: "integer_601".into(),
            normalization: "divide_255".into(),
        };
        assert!(preprocess(b"not image", &p).is_err());
    }
    #[test]
    fn invalid_contract_and_checksum() {
        let metadata: Metadata =
            serde_json::from_str(include_str!("../../tests/fixtures/trained/metadata.json"))
                .unwrap();
        let mut invalid = metadata.clone();
        invalid.preprocessing.layout = "NCHW".into();
        assert!(validate_metadata(&invalid).is_err());
        invalid = metadata.clone();
        invalid.classes.reverse();
        assert!(validate_metadata(&invalid).is_err());
        assert!(Engine::load(metadata, b"corrupted ONNX").is_err());
    }
    #[test]
    fn shared_trained_fixture_parity() {
        let path = std::env::var_os("QC_TEST_MODEL")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../tests/fixtures/trained")
            });
        let metadata: Metadata =
            serde_json::from_slice(&std::fs::read(path.join("metadata.json")).unwrap()).unwrap();
        let fixture: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("rust-parity.json")).unwrap()).unwrap();
        let image = std::fs::read(path.join("parity-input.png")).unwrap();
        let (pixels, _) = preprocess(&image, &metadata.preprocessing).unwrap();
        let expected = fixture["pixels"].as_array().unwrap();
        for (a, b) in pixels.iter().zip(expected) {
            assert!((*a as f64 - b.as_f64().unwrap()).abs() < 1e-7);
        }
        let mut engine =
            Engine::load(metadata, &std::fs::read(path.join("model.onnx")).unwrap()).unwrap();
        let probabilities = engine.run(pixels).unwrap();
        for (a, b) in probabilities
            .iter()
            .zip(fixture["probabilities"].as_array().unwrap())
        {
            assert!((*a as f64 - b.as_f64().unwrap()).abs() < 1e-5);
        }
    }
}
