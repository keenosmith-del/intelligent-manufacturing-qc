"""Held-out evaluation, calibration and failure analysis."""

import numpy as np
from sklearn.metrics import accuracy_score, classification_report, confusion_matrix, f1_score

from .data import CLASSES


def evaluate(y, probabilities, rows):
    if probabilities.shape != (len(y), len(CLASSES)) or not np.isfinite(probabilities).all():
        raise ValueError("invalid probability tensor")
    if not np.allclose(probabilities.sum(axis=1), 1, atol=1e-5):
        raise ValueError("probabilities must sum to one")
    predicted = probabilities.argmax(axis=1)
    confidence = probabilities.max(axis=1)
    report = classification_report(
        y, predicted, labels=range(6), target_names=CLASSES, output_dict=True, zero_division=0
    )
    bins, ece = [], 0.0
    for lower in np.arange(0, 1, 0.1):
        mask = (confidence >= lower) & (confidence < lower + 0.1 + (1e-6 if lower > 0.85 else 0))
        if mask.any():
            accuracy = float((predicted[mask] == y[mask]).mean())
            avg = float(confidence[mask].mean())
            ece += abs(avg - accuracy) * float(mask.mean())
            bins.append(
                {
                    "lower": float(lower),
                    "count": int(mask.sum()),
                    "accuracy": accuracy,
                    "confidence": avg,
                }
            )
    errors = [
        {
            "path": rows[i]["path"],
            "actual": CLASSES[int(y[i])],
            "predicted": CLASSES[int(predicted[i])],
            "confidence": float(confidence[i]),
        }
        for i in range(len(y))
        if predicted[i] != y[i]
    ]
    return {
        "accuracy": float(accuracy_score(y, predicted)),
        "macro_f1": float(f1_score(y, predicted, average="macro", labels=range(6))),
        "weighted_f1": float(f1_score(y, predicted, average="weighted", labels=range(6))),
        "per_class": {c: report[c] for c in CLASSES},
        "confusion_matrix": confusion_matrix(y, predicted, labels=range(6)).tolist(),
        "test_count": len(y),
        "calibration": {"ece": ece, "bins": bins},
        "errors": sorted(errors, key=lambda e: -e["confidence"]),
        "low_confidence": [
            {"path": rows[i]["path"], "confidence": float(confidence[i])}
            for i in range(len(y))
            if confidence[i] < 0.7
        ],
    }
