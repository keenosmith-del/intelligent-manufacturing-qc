#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -f backend/.env ]; then set -a; source backend/.env; set +a; fi
if [ -x .tooling/cargo/bin/cargo ]; then
  export CARGO_HOME="$PWD/.tooling/cargo" RUSTUP_HOME="$PWD/.tooling/rustup"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
export MODEL_ARTIFACT_PATH="${MODEL_ARTIFACT_PATH:-$PWD/artifacts}"
export LOCAL_STORAGE_PATH="${LOCAL_STORAGE_PATH:-$PWD/.storage}"
export ORT_DYLIB_PATH="${ORT_DYLIB_PATH:-$(.venv/bin/python scripts/runtime-path.py)}"
export RUST_LOG="${RUST_LOG:-info}"
cargo run --manifest-path backend/Cargo.toml --locked
