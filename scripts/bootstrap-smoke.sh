#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Smoke plumbing only: all artifacts retain synthetic provenance.
PYTHONPATH=ml/src .venv/bin/python -m qc synthetic --count 36
PYTHONPATH=ml/src .venv/bin/python -m qc prepare --data data/synthetic --provenance data/synthetic/provenance.json
PYTHONPATH=ml/src .venv/bin/python -m qc train --version "${1:-smoke-a}" --epochs 35
