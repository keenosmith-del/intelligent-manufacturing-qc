#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -x .tooling/cargo/bin/cargo ]; then
  export CARGO_HOME="$PWD/.tooling/cargo" RUSTUP_HOME="$PWD/.tooling/rustup"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
export ORT_DYLIB_PATH="${ORT_DYLIB_PATH:-$(.venv/bin/python scripts/runtime-path.py)}"
export QC_TEST_MODEL="$PWD/tests/fixtures/trained"
.venv/bin/ruff check ml scripts
.venv/bin/pytest ml/tests -q
cargo fmt --manifest-path backend/Cargo.toml --check
cargo clippy --manifest-path backend/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path backend/Cargo.toml --locked
cargo build --manifest-path backend/Cargo.toml --release --locked
npm run lint --prefix frontend
npm run test --prefix frontend
npm run build --prefix frontend
