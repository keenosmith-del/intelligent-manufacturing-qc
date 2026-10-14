import json

import numpy as np
import pytest
from PIL import Image

from qc.data import CLASSES, load_split, pixels, prepare, synthetic
from qc.evaluation import evaluate


def test_split_and_deduplication(tmp_path):
    root = tmp_path / "raw"
    provenance = synthetic(root, count=10)
    original = root / CLASSES[0] / "000.png"
    (original.parent / "duplicate.png").write_bytes(original.read_bytes())
    (original.parent / "bad.png").write_bytes(b"broken")
    a = prepare(root, tmp_path / "a", provenance)
    b = prepare(root, tmp_path / "b", provenance)
    assert len(a["records"]) == 60
    assert len(a["duplicates"]) == 1
    assert len(a["rejected"]) == 1
    assert a["dataset_version"] == b["dataset_version"]
    groups = [
        {r["pixel_sha256"] for r in a["records"] if r["split"] == s}
        for s in ["train", "validation", "test"]
    ]
    assert not groups[0] & groups[1] and not groups[0] & groups[2] and not groups[1] & groups[2]
    x, y, _ = load_split(a, "test")
    assert x.shape == (12, 64, 64, 1) and y.shape == (12,)
    original.write_bytes(b"changed")
    with pytest.raises(ValueError, match="changed"):
        load_split(
            a,
            next(
                r["split"]
                for r in a["records"]
                if r["path"].endswith("000.png") and r["label"] == CLASSES[0]
            ),
        )


def test_pixel_contract():
    image = Image.fromarray(
        np.array([[[255, 0, 0], [0, 255, 0]], [[0, 0, 255], [255, 255, 255]]], dtype=np.uint8)
    )
    x = pixels(image, 3, 3)
    assert x.dtype == np.float32
    np.testing.assert_allclose(
        x[:, :, 0], np.array([[76, 76, 150], [76, 76, 150], [29, 29, 255]]) / 255, atol=1e-7
    )


def test_metrics_and_errors():
    y = np.arange(6)
    probs = np.eye(6, dtype=np.float32)
    rows = [{"path": str(i)} for i in y]
    report = evaluate(y, probs, rows)
    assert report["macro_f1"] == 1 and report["accuracy"] == 1
    probs[0] = probs[1]
    report = evaluate(y, probs, rows)
    assert len(report["errors"]) == 1 and report["errors"][0]["actual"] == "crazing"
    with pytest.raises(ValueError):
        evaluate(y, np.zeros((6, 6)), rows)


def test_provenance_and_class_mapping(tmp_path):
    provenance = synthetic(tmp_path / "raw", 7)
    with pytest.raises(ValueError, match="provenance"):
        prepare(tmp_path / "raw", tmp_path / "prepared", {})
    report = prepare(tmp_path / "raw", tmp_path / "prepared", provenance)
    assert report["classes"] == CLASSES
    assert json.loads((tmp_path / "prepared" / "manifest.json").read_text())["provenance"][
        "synthetic"
    ]
