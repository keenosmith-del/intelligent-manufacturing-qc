# Development log

## Phase 1 — Architecture and foundations (2026-10-14)

Inspected the initial repository: only README.md exists; no applicable AGENTS.md was found. No Git operations are authorized. The implementation uses React/Vite, Axum/ONNX Runtime, Python TensorFlow tooling and a focused Firestore REST adapter. Images and artifacts remain outside Firestore. Planned contract: six ordered defect classes, float32 NHWC grayscale input, normalized to [0,1], softmax output. Model releases use a persisted active pointer and a serialized local release coordinator.

Created primary module boundaries, documentation and ignore rules. Found Node 24 and Python 3.12; Rust and Java were absent. Docker daemon is available when accessed outside the sandbox. Requested dependency downloads and a repository-local Rust toolchain. Verification is pending installation. NEU official source was inspected: it offers NEU-CLS and asks for citations, but does not establish clear redistribution rights. No third-party images have been acquired. Synthetic smoke data will be visibly identified throughout the system; real dataset acquisition stays manual pending rights verification.

## Phase 2 — Dataset and TensorFlow training

Implemented `ml/src/qc/data.py`, `evaluation.py`, `train.py` and CLI, baseline config and four tests. Preparation validates files and provenance, removes decoded-pixel duplicates, creates seeded stratified splits, hashes content and exports JSON/Markdown reports. Actual compact CNN training uses training-only augmentation, Adam, early stopping and saved best checkpoints. Evaluation includes held-out confusion matrix, per-class metrics, calibration bins and error examples.

Executed `.venv/bin/pytest ml/tests -q`: 4 passed. Executed Ruff: all checks passed. Installed pinned TensorFlow 2.16.2 / tf2onnx 1.16.1 / ONNX 1.17.0 / ONNX Runtime 1.22.0. Synthetic run: 216 unique images, 150 train, 30 validation, 36 test. Twelve epochs completed; macro-F1 0.555556 and accuracy 0.666667 are smoke-only figures. No industrial dataset or industrial quality claim. Full NEU path uses the same pipeline after manual rights-verified acquisition.

## Phase 3 — Export and Rust inference (in progress)

Exported actual best checkpoint to opset 17; ONNX checker and executable input/output validation passed. TensorFlow/ONNX parity: all 36 test classifications matched, maximum absolute probability difference 1.7881393432617188e-7, tolerance 1e-5. Package includes checksums, class/preprocessing files, manifest, configuration, evaluation, parity and shared Rust fixture. Implemented Rust integer grayscale/nearest resize contract and cached runtime session, blocking-pool inference, decoded format and dimension checks. Rust verification is pending dependency compilation.

### Phase 3 verification update

Rust dependency download completed; `cargo check --offline` passed after fixing decimal literals and a readiness borrow. A second actual training package, smoke-a, completed with early stopping after 25 epochs (best validation checkpoint at epoch 20). Held-out synthetic accuracy and macro-F1 are 1.0; parity max error 3.5762786865234375e-7 on 36 test images. This result reflects easy procedural patterns, not industrial quality. More independently trained smoke versions are being generated for release tests.

## Phase 4 — Persistence and inspection APIs

Implemented `backend/src/firestore.rs`, `storage.rs`, `routes.rs`, typed records and registry integration. Firestore adapter recursively converts JSON fields and consumes list pagination. Atomic commits preserve review history, original predictions and release pointers. Local image writes use temporary files and rename. GCS uses JSON/media REST endpoints and Cloud Run metadata credentials. Request validation limits images to 10 MiB and 16 megapixels, supports actual decoded PNG/JPEG/BMP, generates safe UUID object names, restricts CORS and protects mutations with an operator bearer token. Inference runs in Tokio's blocking pool through a cached serialized session.

Inspections persist only after real inference and image storage succeed; persistence errors trigger image cleanup. Lost storage/DB connectivity produces explicit errors. Core unit tests and emulator integration are in progress. Cloud credentials are unavailable; cloud adapters are implemented but not verified against GCP.

## Phase 5 — Frontend application (in progress)

Implemented React Router navigation, Overview, uploads and image preview, filtered/paginated history, inspection detail, human review, Model Lab and Monitoring. Centralized Fetch adapter and typed responses. The cream/charcoal responsive interface displays synthetic provenance prominently, includes loading/error/empty states and shows no pass/fail claims. Operator token stays in page memory. Frontend production build has reached Vite compilation; UI workflow tests follow.

### Phase 4–5 verification update

Frontend tests: 6 passed, covering successful upload/prediction, oversized upload validation, history/filter requests, preserved prediction during review, backend error/retry and model selection/rejection. `npm run lint`: passed. `npm run build`: passed (TypeScript and production assets).

The Firestore emulator started successfully in Docker and answered localhost health. Requested API/frontend ports 5050/5173 are occupied by pre-existing Node processes; verification uses 5051/5174 without interrupting them. Rust ONNX assertions passed on the original rc.10 binding but the process aborted on macOS exit. Upstream release notes identify an environment lifetime fix. Upgraded to ort/ort-sys 2.0.0-rc.11 and ONNX Runtime 1.23.2; repeat verification is in progress. Python tests with the new runtime: 6 passed, including executable graph/checksum/shared pixels and probability fixture; Ruff passed. One upstream protobuf deprecation warning remains.

## Phase 6 — Model Lab and MLOps

Implemented full metadata/evaluation/class/confusion/training-history display, compatible version comparison, actual registry import, release checks, hot activation and rollback. Monitoring exposes real persisted class/confidence/latency/review aggregates, active-model PSI, process upload error counters and snapshots. Single-process release lock and persisted active pointer ensure local coordinated swaps; no horizontal deployment claim. Generated independently trained smoke-b (seed 43), early-stopped at epoch 32, best checkpoint epoch 27. Smoke-only test accuracy/macro-F1 1.0; TF/ONNX max error 4.76837158203125e-7. Lifecycle API verification follows.

### Phase 6 integration verification

`api-smoke.py --url http://localhost:5051 --source artifacts/smoke-a --second-source artifacts/smoke-b` passed. Executed real cached Rust ONNX inference, image retrieval, Firestore detail/history persistence, correction with immutable prediction, invalid review/image rejection, unauthorized registry rejection, second trained model activation, failing-gate rejection without pointer change, rollback, 30 additional inspections, PSI, snapshot persistence and release history assertions. Run created 33 inspections; measured mean inference 0.307032 ms, p95 0.469625 ms on this synthetic image and local machine. These are local smoke timings, not production throughput claims.

Resolved Firestore rejection of direct nested arrays by wrapping nested arrays as reserved maps (confusion matrices). Commit REST path verified against official API; atomically writes `/documents:commit`. Browser verification caught aliasing of `import.meta.env` preventing Vite substitution; corrected to direct access. Mocked component tests could not detect this bundler-specific defect. Browser re-verification pending.

## Phase 7 — CI/CD and containers (in progress)

Added application CI (frontend lint/typechecks/tests/build, Python tests and artifact contract, Rust format/Clippy/tests/release build, emulator API lifecycle and container builds), separate manual synthetic training/export workflow and optional approved GCP OIDC/WIF deployment. No workflow was executed on GitHub. Container image built locally successfully with non-root runtime and explicitly packaged ONNX Runtime 1.23.2; repeat build is required after the Firestore correction. Compose uses a dedicated named storage volume and project namespace to avoid non-root bind-mount permissions and mixing direct/local model pointers. Production deployment remains private via Cloud Run IAM with one instance and a Secret Manager operator token. GCP infrastructure is not accessed.

## Phase 8 — Full verification and documentation (in progress)

Combined `scripts/check.sh` completed successfully: Python Ruff + 6 tests, Rust format/Clippy + 9 tests + release build, frontend ESLint + 6 tests + TypeScript/Vite production build. Independent held-out parity revalidation against Runtime 1.23.2 passed on 36 images (max error 4.768372e-7); offline ONNX reevaluation and candidate regression check passed. Native API lifecycle and non-root Docker lifecycle passed with 33 inspections each. Added read-only bounded Firestore retries and post-timeout inspection commit reconciliation; no mutation retry can silently duplicate a review/release.

Browser verified responsive overview and history, actual class/provenance/evaluation/confusion/training curves, and saved a correction. After page refresh, human label remained Scratches while original model Crazing/96.1% stayed unchanged. Screenshots saved under docs/. Browser file chooser failed at the tool boundary; browser upload is unverified, while API and frontend component upload tests passed. Kept the tool limitation explicit.

Recorded 20-request/4-worker real API benchmark on repeated procedural image: 100.59 requests/sec; end-to-end mean 37.46 ms / p95 87.22 ms; inference mean .80313 ms. Not a sustained or industrial performance claim. Added actual undertrained smoke-v1 package to CI fixtures for rejection using measured recalls/F1, replacing tampered negative-test reports in the final script. Final release binary lifecycle rerun passed with two independently trained models and actual undertrained rejection; persisted active pointer was restored at startup.

Documentation now includes setup, API, data/storage, ML, MLOps, testing and deployment, with commands and genuine limitations. No Git staging, commits, pushes, merges or history changes. GCP and real NEU training remain unverified for the stated external reasons.

### Final local delivery audit

Pinned the actually used emulator image digest and added documented Compose `API_PORT`/`FRONTEND_PORT` overrides. Changing the emulator image reference recreated the memory-only emulator and cleared only this session's smoke records, as documented; generated objects/packages were retained. Seeded and verified fresh final Compose records with actual trained/undertrained packages. Final Compose lifecycle passed: 33 inspections, measured mean inference .285458 ms / p95 .323500 ms, actual undertrained class-recall/regression rejection, review, rollback and monitoring. Restarted only the API container; `/ready` restored the same persisted version and executed inference successfully. Refreshed just the frontend after the final previous-model/correction-rate display. Recorded structured outcomes in docs/verification/results.json.

The verified local deliverable is complete. Documentation and reproducible scripts identify every external/unverified limit. Default ports remain unchanged; running Compose uses :5176 frontend / :5052 API / :8080 emulator to preserve unrelated processes. Stopped this task's temporary native API/Vite sessions; retained the integrated Compose app. No Git index or history mutations were performed.
