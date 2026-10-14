"""Re-evaluate a registered-style ONNX package without training or online serving."""

import json
from pathlib import Path

import onnxruntime as ort

from .data import digest, load_split
from .evaluation import evaluate


def reevaluate(package, manifest_path, output):
    root = Path(package)
    metadata = json.loads((root / "metadata.json").read_text())
    manifest = json.loads(Path(manifest_path).read_text())
    if digest(root / "model.onnx") != metadata["sha256"]:
        raise ValueError("model checksum mismatch")
    if manifest["dataset_version"] != metadata["dataset_version"]:
        raise ValueError("dataset version mismatch; choose an explicit new evaluation baseline")
    x, y, rows = load_split(manifest, "test")
    session = ort.InferenceSession(str(root / "model.onnx"), providers=["CPUExecutionProvider"])
    probabilities = session.run(None, {session.get_inputs()[0].name: x})[0]
    report = evaluate(y, probabilities, rows)
    report.update(
        version=metadata["version"],
        dataset_version=manifest["dataset_version"],
        synthetic=manifest["provenance"]["synthetic"],
        runtime=ort.__version__,
    )
    Path(output).write_text(json.dumps(report, indent=2))
    return report
