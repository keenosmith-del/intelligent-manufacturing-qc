import argparse
import json
from pathlib import Path

from .data import prepare, synthetic

parser = argparse.ArgumentParser(description="Manufacturing QC offline ML tooling")
sub = parser.add_subparsers(dest="command", required=True)
gen = sub.add_parser("synthetic", help="Generate procedural smoke images, NOT industrial data")
gen.add_argument("--output", default="data/synthetic")
gen.add_argument("--count", type=int, default=24)
prep = sub.add_parser("prepare")
prep.add_argument("--data", required=True)
prep.add_argument("--provenance", required=True)
prep.add_argument("--output", default="data/prepared")
prep.add_argument("--seed", type=int, default=42)
prep.add_argument("--width", type=int, default=64)
prep.add_argument("--height", type=int, default=64)
training = sub.add_parser("train")
training.add_argument("--manifest", default="data/prepared/manifest.json")
training.add_argument("--config", default="ml/configs/baseline.json")
training.add_argument("--output", default="artifacts")
training.add_argument("--version", required=True)
training.add_argument("--epochs", type=int)
evaluation = sub.add_parser("evaluate")
evaluation.add_argument("--package", required=True)
evaluation.add_argument("--manifest", default="data/prepared/manifest.json")
evaluation.add_argument("--output", default="artifacts/reevaluation.json")
args = parser.parse_args()
if args.command == "synthetic":
    synthetic(args.output, args.count)
elif args.command == "prepare":
    prepare(
        args.data,
        args.output,
        json.loads(Path(args.provenance).read_text()),
        args.seed,
        args.width,
        args.height,
    )
elif args.command == "evaluate":
    from .offline import reevaluate

    print(json.dumps(reevaluate(args.package, args.manifest, args.output), indent=2))
else:
    from .train import train

    config = json.loads(Path(args.config).read_text())
    if args.epochs:
        config["epochs"] = args.epochs
    train(args.manifest, config, args.output, args.version)
