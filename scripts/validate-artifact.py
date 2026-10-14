"""Independent graph/checksum/runtime and optional full held-out TensorFlow parity verification."""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ml/src"))
from qc.data import CLASSES, digest, load_split, pixels  # noqa: E402

parser = argparse.ArgumentParser()
parser.add_argument("package")
parser.add_argument("--tensorflow", action="store_true")
args = parser.parse_args()
root = Path(args.package)
metadata = json.loads((root / "metadata.json").read_text())
assert metadata["classes"] == CLASSES
assert digest(root / "model.onnx") == metadata["sha256"]
assert metadata["parity"]["passed"] and metadata["parity"]["max_absolute_error"] <= 1e-5
onnx.checker.check_model(onnx.load(root / "model.onnx"))
session = ort.InferenceSession(
    str(root / "model.onnx"), providers=["CPUExecutionProvider"]
)
if args.tensorflow:
    import tensorflow as tf

    manifest = json.loads((root / "manifest.json").read_text())
    x, _, _ = load_split(manifest, "test")
    model = tf.keras.models.load_model(root / "best.keras")
    expected = model(x, training=False).numpy()
else:
    fixture = json.loads((root / "rust-parity.json").read_text())
    with Image.open(root / "parity-input.png") as image:
        prep = metadata["preprocessing"]
        x = pixels(image, prep["width"], prep["height"])[None]
    expected = np.array([fixture["probabilities"]], dtype=np.float32)
actual = session.run(None, {session.get_inputs()[0].name: x})[0]
assert actual.shape == expected.shape == (len(x), 6)
assert np.isfinite(actual).all()
assert np.allclose(actual.sum(1), 1, atol=1e-5)
assert np.array_equal(actual.argmax(1), expected.argmax(1))
np.testing.assert_allclose(actual, expected, atol=1e-5, rtol=0)
print(
    json.dumps(
        {
            "passed": True,
            "samples": len(x),
            "max_absolute_error": float(np.max(np.abs(actual - expected))),
            "runtime": ort.__version__,
        },
        indent=2,
    )
)
