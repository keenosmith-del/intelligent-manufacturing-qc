import hashlib
import json
from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
from PIL import Image

from qc.data import CLASSES, pixels

ROOT = Path(__file__).resolve().parents[2] / "tests/fixtures/trained"


def test_executable_onnx_contract_and_checksum():
    metadata = json.loads((ROOT / "metadata.json").read_text())
    assert metadata["classes"] == CLASSES
    assert metadata["provenance"]["synthetic"] is True
    assert hashlib.sha256((ROOT / "model.onnx").read_bytes()).hexdigest() == metadata["sha256"]
    onnx.checker.check_model(onnx.load(ROOT / "model.onnx"))
    session = ort.InferenceSession(str(ROOT / "model.onnx"), providers=["CPUExecutionProvider"])
    with Image.open(ROOT / "parity-input.png") as image:
        x = pixels(image, 64, 64)
    fixture = json.loads((ROOT / "rust-parity.json").read_text())
    np.testing.assert_allclose(x.reshape(-1), fixture["pixels"], atol=1e-7)
    probabilities = session.run(None, {session.get_inputs()[0].name: x[None]})[0]
    assert probabilities.shape == (1, 6)
    assert np.isfinite(probabilities).all()
    np.testing.assert_allclose(probabilities[0], fixture["probabilities"], atol=1e-5)
    np.testing.assert_allclose(probabilities.sum(), 1, atol=1e-5)


def test_recorded_export_report_contract():
    # TensorFlow checkpoint is generated, not bundled; cross-runtime baseline probabilities
    # were verified during actual training and are retained in the fixture report.
    report = json.loads((ROOT / "parity.json").read_text())
    assert report["passed"] and report["same_classes"]
    assert report["sample_count"] == 36
    assert report["max_absolute_error"] <= report["tolerance"] == 1e-5
