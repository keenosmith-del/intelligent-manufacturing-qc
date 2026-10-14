"""Offline counterpart to Rust release checks; no promotion or metric fabrication."""

import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("candidate")
parser.add_argument("--baseline")
parser.add_argument("--min-f1", type=float, default=0.5)
parser.add_argument("--min-recall", type=float, default=0.2)
parser.add_argument("--max-regression", type=float, default=0.03)
args = parser.parse_args()
candidate = json.loads(Path(args.candidate).read_text())
reasons = []
if candidate["evaluation"]["macro_f1"] < args.min_f1:
    reasons.append("Macro-F1 below threshold")
for label in candidate["classes"]:
    if candidate["evaluation"]["per_class"][label]["recall"] < args.min_recall:
        reasons.append("Recall below threshold: " + label)
if (
    not candidate["parity"]["passed"]
    or candidate["parity"]["max_absolute_error"] > 1e-5
):
    reasons.append("Export parity failed")
if args.baseline:
    baseline = json.loads(Path(args.baseline).read_text())
    if any(
        candidate[key] != baseline[key]
        for key in ("dataset_version", "test_split_hash", "classes")
    ):
        reasons.append("Incompatible evaluation dataset or split")
    elif (
        candidate["evaluation"]["macro_f1"] + args.max_regression
        < baseline["evaluation"]["macro_f1"]
    ):
        reasons.append("Macro-F1 regression exceeds threshold")
result = {"passed": not reasons, "reasons": reasons, "candidate": candidate["version"]}
print(json.dumps(result, indent=2))
raise SystemExit(0 if result["passed"] else 1)
