# Version 1 delivery report — 2026-10-14

## Delivered application

Implemented an integrated locally runnable manufacturing surface-pattern classification platform in the existing repository: React UI, Rust main API and ONNX serving, offline TensorFlow training, Firestore persistence, local/GCS object stores, human review, registry/comparison, release gates, activation/rollback, monitoring, Docker and CI/CD. Online outputs come from actual trained weights; no simulated inference or secondary backend exists. Work is uncommitted, with no Git staging/history operations.

This is a known-defect classifier, not a validated industrial pass/fail system. Local software workflows were verified on project-owned synthetic images. Real NEU training and cloud deployment remain unverified for explicit external reasons below.

## Final directory structure and responsibilities

```text
intelligent-manufacturing-qc/
├── frontend/
│   ├── src/
│   │   ├── pages/              Overview, Inspections, Detail, Models, Monitoring
│   │   ├── components/         Shared UI/resource hook; model evaluation/curves
│   │   ├── services/api.ts     Fetch and memory-only operator token
│   │   ├── types/index.ts      API data contracts
│   │   ├── App.tsx             Router and responsive navigation
│   │   ├── main.tsx            React entry
│   │   └── styles.css          Cream/charcoal Tailwind + product styles
│   ├── package.json / package-lock.json
│   ├── vite.config.ts / tsconfig.json / eslint.config.js
│   ├── tailwind.config.js / postcss.config.js
│   └── Dockerfile / nginx.conf / .env.example
├── backend/
│   ├── src/
│   │   ├── main.rs             Runtime/startup/restore/server/shutdown
│   │   ├── config.rs           Central environment validation
│   │   ├── error.rs            Structured HTTP errors
│   │   ├── models.rs           Typed records and fixed class mapping
│   │   ├── routes.rs           Main REST endpoints and request validation
│   │   ├── inference.rs        Decode/preprocess/cached ONNX session
│   │   ├── firestore.rs        Typed REST serialization/pagination/atomic commits
│   │   ├── storage.rs          Safe local/GCS image and package operations
│   │   ├── registry.rs         Import/quality gates/activation/rollback
│   │   └── monitoring.rs       Pure observed-data aggregation and PSI
│   └── Cargo.toml / Cargo.lock / Dockerfile / .env.example
├── ml/
│   ├── src/qc/
│   │   ├── data.py             Validation/deduplication/splits/pixel contract
│   │   ├── train.py            TensorFlow CNN/checkpoint/evaluate/export/parity
│   │   ├── evaluation.py       Metrics/calibration/failure analysis
│   │   ├── offline.py          Independent ONNX reevaluation
│   │   └── __main__.py         Reproducible offline CLI
│   ├── configs/                Baseline and alternative smoke seed
│   ├── tests/                  Dataset, metrics and executable artifact tests
│   └── requirements.txt / requirements-lock.txt / pyproject.toml
├── scripts/                    Setup/run/check, validation, API lifecycle, benchmark
├── tests/fixtures/              Actual small trained/undertrained ONNX packages
├── docs/                       Engineering documentation, screenshots, verification
├── .github/workflows/          Application, manual ML, optional approved GCP delivery
├── docker-compose.yml / .dockerignore / .gitignore
└── README.md
```

Ignored generated directories: `.venv`, `.tooling`, `data`, `artifacts`, `.storage`, Rust target and frontend node_modules/dist. Each package carries provenance; small CI fixtures are intentionally retained and contain no third-party imagery.

## REST API

GET `/health`, `/ready`, `/api/classes`, `/api/overview`; GET/POST `/api/inspections`; GET `/api/inspections/{id}` and `/image`; GET/POST `/api/inspections/{id}/reviews`; GET/POST `/api/models`; POST `/api/models/{version}/validate` and `/activate`; POST `/api/models/rollback`; GET `/api/training-runs`, `/api/releases`, `/api/monitoring`; GET/POST `/api/monitoring/snapshots`.

[API documentation](api.md) specifies payloads, responses, limits, errors and example curl commands. Operator token protects registration, reviews, releases and snapshots. Uploads validate real PNG/JPEG/BMP bytes, <=10 MiB and <=16 megapixels; CORS is restricted. Cloud must also enforce authenticated service access.

## Dataset, model and actual evaluation

Intended real source: [official NEU-CLS distribution](https://faculty.neu.edu.cn/songkc/en/zdylm/263270/list/index.htm), 1,800 six-class grayscale images. The source asks for attribution but does not establish explicit redistribution terms; no images were acquired/redistributed. Real-data pipeline accepts licensed local six-class folders and provenance.

Executed data: **216 procedural project-owned smoke images**, 36/class; 150 train, 30 validation, 36 test. Ordered classes: crazing, inclusion, patches, pitted_surface, rolled_in_scale, scratches. Exact content/decoded-pixel deduplication before deterministic stratified splitting. Dataset version and test split are hashed. No industrial accuracy claim is supported.

CNN: grayscale 64×64 input → training-only flip/rotation → Conv16/max-pool → Conv32/max-pool → Conv64/global average pool → Dense32 → six-way softmax. Adam .001, sparse categorical cross-entropy, batch32, patience5; seeds42/43. TensorFlow2.16.2, Keras3.15.1, tf2onnx1.16.1, ONNX1.17.0, opset17; current Python/Rust Runtime1.23.2 and Rust ort rc.11.

| Actual run | Epochs | Test accuracy | Macro-F1 | Export max TF/ONNX error |
|---|---:|---:|---:|---:|
| smoke-v1 | 12 | .666667 | .555556 | 1.788139e-7 |
| smoke-a | 25 (early stop) | 1.0 | 1.0 | 3.576279e-7 |
| smoke-b | 32 (early stop) | 1.0 | 1.0 | 4.768372e-7 |

All figures are synthetic smoke results only. Actual confusion matrices, per-class precision/recall/F1, weighted-F1, validation loss, calibration and error cases are in package evaluation reports; training histories/configurations are preserved. Actual smoke-v1 quality fails required class recall and baseline regression gates.

## Export, Rust inference and storage

ONNX graph checks and shape/output checks passed for all three actual exports. All 36 test argmax classifications matched TensorFlow at export. After Runtime upgrade to1.23.2, independent full smoke-a revalidation passed with max absolute error4.768372e-7 (tolerance1e-5). Rust fixture verifies exact Python/Rust preprocessed pixels and actual probabilities. JPEG library rounding remains a deployment parity caveat.

Rust loads weights once, checks SHA-256, executes a shape/output probe and runs requests in Tokio's blocking pool behind a serialized cached session. It decodes dimensions before full allocation and uses the same integer grayscale/nearest coordinate/normalization NHWC contract as Python. Failed inference returns explicit error.

Firestore REST is the primary store; emulator tested. Typed adapter consumes pagination, wraps unsupported nested arrays, retries bounded idempotent reads and commits related writes atomically. Local images/weights use safe generated keys and atomic writes. GCS JSON/media APIs and Cloud Run metadata tokens are implemented, **not cloud-tested**. Post-timeout inspection reconciliation avoids blindly deleting potentially committed evidence. Ambiguous/orphan outcomes remain an operator concern.

## Registry and release lifecycle

Registered versions include training/evaluation/export/data metadata. State is candidate → validated/rejected; active/previous pointer is separately persisted. Gates check actual executable artifact, checksum, mapping/preprocessing, export parity, macro-F1/per-class recall and compatible baseline regression. Incompatible dataset/split comparison is refused. Synthetic activation requires explicit emulator-only configuration.

Activation constructs a working session, commits pointer/release/audit, then hot swaps. In-flight requests retain their actual captured model version. Rollback loads the previous validated package. Restart restores persisted active state. Single-process coordination is intentional; horizontal release/review synchronization is not implemented.

## CI/CD and actual checks

Application workflow: frontend install/lint/typecheck/tests/build; Python lint/tests/artifact contracts; Rust format/Clippy/tests/release build; real emulator API lifecycle; backend/frontend container builds. Manual ML workflow: validate/prepare/train/evaluate/export/parity/package plus quality/regression checks. Optional manual deployment: Artifact Registry + private single-instance Cloud Run, GitHub OIDC/WIF, Secret Manager token and an approved configured environment. No long-lived service-account JSON keys. **GitHub Actions and GCP deployment were not executed remotely.**

Executed: 6 Python + 9 Rust + 6 frontend tests; Ruff/rustfmt/Clippy/ESLint; TypeScript/Vite production build; Rust release build; actual TensorFlow training/export; Python/Rust parity; native API lifecycle; non-root container lifecycle; full Compose startup and frontend SPA serving; persisted review refresh and model restoration. [Testing](testing.md) records commands/results/failures/coverage and [verification/results.json](verification/results.json) stores measured run summaries.

Local 20-request/4-worker benchmark on one repeated synthetic image: 100.59 requests/sec, end-to-end mean37.46ms/p95 87.22ms. This brief local emulator-backed smoke is not a sustained industrial benchmark.

## Exact local commands

From repository root:

```sh
python3.12 -m venv .venv
.venv/bin/python -m pip install -r ml/requirements.txt
npm ci --prefix frontend
cp backend/.env.example backend/.env
# Edit token, root-relative paths and explicit local ALLOW_SYNTHETIC=true.
export ADMIN_TOKEN='replace-with-a-long-local-operator-token'
docker compose up -d firestore
./scripts/bootstrap-smoke.sh smoke-a
./scripts/serve-api.sh
# Separate terminal:
npm run dev --prefix frontend
```

Then enter token in Operator access, register/activate smoke-a in Model Lab and upload/review an image. All detailed first-run/export/register/curl/real-data commands: [setup](setup.md). Full checks: `./scripts/check.sh`. No-training integration: `.venv/bin/python scripts/api-smoke.py` with the same exported token and running API. Container mode: `ADMIN_TOKEN="$ADMIN_TOKEN" docker compose up --build -d`. If default ports are busy, `API_PORT=5052 FRONTEND_PORT=5176 ADMIN_TOKEN="$ADMIN_TOKEN" docker compose up --build -d`.

## Documentation and remaining limits

Created/updated README, development-log, architecture, setup, API, machine-learning, MLOps, data-management, testing and deployment documents, this delivery report, verification JSON and actual browser screenshots. Development log records implementations, decisions, dependencies, tests and resolved failures throughout.

Outstanding: NEU rights and real industrial training/evaluation; GCP credentials/infrastructure/IAM/WIF and frontend cloud authentication; remote CI runs; multi-instance synchronization; sustained production load/failure testing; near-duplicate/source-batch leakage detection; image-feature drift; automated retention/orphan reconciliation. Browser chooser automation failed at the tool boundary; browser upload itself remains unverified, while API upload/component tests and browser persisted review passed. Local auth is single-operator, not production role management. Default ports were occupied by unrelated Node processes and were preserved; testing used free ports.
