# Local setup (macOS)

Run commands from the repository root unless indicated. Verified on Apple Silicon, Python 3.12.15, Node 24.21.0, Rust 1.99.0 and Docker Desktop 29.8.2. Python 3.12 is required for the pinned TensorFlow wheel. CPU is the baseline; no Metal/GPU package is required. Use a Rust toolchain with rustfmt/Clippy (`rustup component add rustfmt clippy`). This implementation session installed a repository-local toolchain under ignored `.tooling`; normal clones should install Rust from rustup's official source.

```sh
python3.12 -m venv .venv
.venv/bin/python -m pip install -r ml/requirements.txt
npm ci --prefix frontend
cp backend/.env.example backend/.env
```

Edit backend/.env: set a long operator token; `MODEL_ARTIFACT_PATH=artifacts` and `LOCAL_STORAGE_PATH=.storage` when using the root serve script. `.env` is a trusted local shell configuration, never committed. Set `ALLOW_SYNTHETIC=true` only for smoke/emulator mode. Frontend operator access accepts the same token and keeps it in memory. In another terminal, also export ADMIN_TOKEN for CLI/Compose:

```sh
export ADMIN_TOKEN='replace-with-a-long-local-operator-token'
docker compose up -d firestore
```

Without Docker, install Java and gcloud's Firestore emulator component, then run `gcloud beta emulators firestore start --host-port=127.0.0.1:8080 --project=manufacturing-qc-local`. No Java was installed on this Mac; the Docker emulator path was verified.

Train actual TensorFlow weights on procedural smoke data (does not establish industrial model quality):

```sh
./scripts/bootstrap-smoke.sh smoke-a
PYTHONPATH=ml/src .venv/bin/python -m qc train --version smoke-b --config ml/configs/smoke-b.json --epochs 35
.venv/bin/python scripts/validate-artifact.py artifacts/smoke-a --tensorflow
```

For a very short training plumbing check, generate/prep as above and use `python -m qc train --version short-smoke --epochs 2` with `PYTHONPATH=ml/src`. Such a model may correctly fail release quality gates; the shipped CI fixture can verify online serving without waiting for training.

Start the API and frontend in separate terminals:

```sh
./scripts/serve-api.sh
npm run dev --prefix frontend
```

Default URLs: frontend http://localhost:5173, API http://localhost:5050, emulator 127.0.0.1:8080. No session starts with fabricated predictions: a missing active model produces readiness 503 until registration/activation. Register and activate through Model Lab after entering the operator token, or:

```sh
curl -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"version":"smoke-a"}' http://localhost:5050/api/models
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" http://localhost:5050/api/models/smoke-a/activate
curl http://localhost:5050/ready
curl -F image=@data/synthetic/scratches/000.png http://localhost:5050/api/inspections
```

Open Inspections, upload an image, inspect probabilities/version, enter operator token, accept/correct in detail, and refresh history. Registry, reviews and snapshots require operator access. The token does not provide multi-user roles or cloud browser sign-in.

No-training fixture path, explicitly synthetic and project-owned:

```sh
mkdir -p artifacts
ADMIN_TOKEN="$ADMIN_TOKEN" .venv/bin/python scripts/api-smoke.py
```

`api-smoke.py` needs an already running API and emulator; it registers unique fixture versions, runs real inference/reviews/releases and leaves auditable smoke records. By default uses the small trained fixture; `--source artifacts/smoke-a --second-source artifacts/smoke-b` checks independently trained versions.

Real NEU data: follow machine-learning.md, verify usage rights, arrange six class folders, write provenance.json, then:

```sh
PYTHONPATH=ml/src .venv/bin/python -m qc prepare --data data/neu --provenance data/neu/provenance.json --output data/neu-prepared
PYTHONPATH=ml/src .venv/bin/python -m qc train --manifest data/neu-prepared/manifest.json --version neu-baseline-v1
PYTHONPATH=ml/src .venv/bin/python -m qc evaluate --package artifacts/neu-baseline-v1 --manifest data/neu-prepared/manifest.json --output artifacts/neu-reevaluation.json
```

Registration uses safe version folders and copies verified artifacts into local/GCS storage. Never reuse a registered version ID for different weights. A fresh emulator loses pointers/history; re-register packages. Existing stored active pointer is restored when only the API restarts.

## Configuration

`PORT` 5050; `BIND_ADDRESS` 127.0.0.1; `FIRESTORE_PROJECT_ID`; `FIRESTORE_EMULATOR_HOST` host:port (unset in cloud); `STORAGE_PROVIDER` local/gcs; `LOCAL_STORAGE_PATH`; `MODEL_ARTIFACT_PATH` directory of version packages; `GCS_BUCKET` required in cloud; `CORS_ORIGIN`; `ADMIN_TOKEN` >=16 bytes; `ALLOW_SYNTHETIC` false by default; `MIN_MACRO_F1`, `MIN_CLASS_RECALL`, `MAX_F1_REGRESSION` each 0..1; `ORT_DYLIB_PATH`; `RUST_LOG`; frontend `VITE_API_URL` (build-time). `scripts/runtime-path.py` locates the compatible runtime library; serve/check scripts set it automatically.

No `.env` loader library is used: serve-api.sh sources backend/.env explicitly. Direct `cargo run` requires exporting variables yourself. Relative paths depend on working directory; root scripts run from root, backend/.env.example shows backend-directory defaults. Cloud Run supplies variables/metadata tokens through deployment configuration.

## Troubleshooting

- Port in use: do not stop unrelated projects. For Compose use `API_PORT=5052 FRONTEND_PORT=5176 ADMIN_TOKEN="$ADMIN_TOKEN" docker compose up --build -d`. This session's defaults were occupied; verification used API 5051, frontend 5175 and container API 5052. Override `PORT=5051` (in backend/.env) and start frontend with `VITE_API_URL=http://localhost:5051 npm run dev --prefix frontend -- --port 5175`; set `CORS_ORIGIN=http://localhost:5175`. Vite strictPort avoids silently changing ports.
- Native ONNX exit crash: use pinned ort rc.11 and ONNX Runtime 1.23.2; rc.10 was reproduced crashing on macOS teardown. Set the exact shared library path, not the Python extension module.
- Model missing/readiness 503: start emulator, import artifact, validate and activate. Inspect API structured logs; no mock fallback exists.
- Firestore unavailable: verify emulator health with `curl http://localhost:8080/`, namespace and port. Restarting emulator discards records; API restart does not.
- Gate rejection: inspect per-class recalls/reasons. Short training can fail legitimately. No override is offered.
- Dataset hash failure: raw files changed since preparation; prepare a new immutable dataset version.
- Missing wheel: use Python 3.12 on a supported architecture. A full environment lock from the verified Mac is recorded in requirements-lock.txt; requirements.txt is the cross-platform primary contract.
