# Executed verification

Verification on 2026-10-14, Apple Silicon macOS, Python 3.12, Node 24, Rust 1.99.0. Generated artifacts/data, libraries and local uploads are ignored; project-owned small trained ONNX fixtures are retained for reproducible CI.

| Check | Actual result |
|---|---|
| Python dataset/metrics/artifact tests | 6 passed; one upstream protobuf deprecation warning |
| Python Ruff | Passed |
| TensorFlow training | Three runs completed: smoke-v1 12, smoke-a 25 and smoke-b 32 epochs |
| ONNX graph + full held-out export parity | Passed for all three; 36 samples per run |
| Independent TF/ONNX Runtime 1.23.2 revalidation | Passed, smoke-a max probability error 4.768372e-7 |
| Rust unit + actual ONNX/shared preprocessing fixture | 9 passed, including normal process exit after binding upgrade |
| Rust rustfmt / Clippy -D warnings | Passed |
| Rust release build | Passed |
| Frontend workflow tests | 6 passed |
| Frontend ESLint / TypeScript / Vite build | Passed |
| Native API + real Firestore emulator integration | Passed: inspection/image/history/review, release/activation/rejection/rollback, PSI/snapshots |
| Non-root backend container + real emulator integration | Passed, same lifecycle on :5052 |
| Full Compose startup, SPA route and API restart restoration | Passed on :5052/:5176; executable readiness restored previous active version |
| Browser responsive overview/history/model pages | Visually verified at narrow default and 1280×900 widths |
| Browser human review + page refresh | Saved correction persisted; original prediction unchanged |
| Browser chooser upload | Tool failed selecting a local image; not verified by browser automation. API upload and component upload passed |
| GitHub workflows | Written; local constituent checks executed, not run on GitHub |
| Cloud GCS/Firestore/Cloud Run/IAM/WIF | Implemented/configurable, not deployed or verified; no credentials |
| Real NEU dataset training | Not performed: usage/redistribution rights unresolved; no third-party images acquired |

`./scripts/check.sh` runs Python lint/tests, Rust format/lint/tests/release build and frontend lint/tests/build. It locates the ONNX Runtime library and enables the actual trained CI fixture. `QC_TEST_MODEL` can override the default fixture; the Rust inference test otherwise uses the bundled fixture, so an ordinary test run does not silently skip inference.

```sh
./scripts/check.sh
.venv/bin/python scripts/validate-artifact.py artifacts/smoke-a --tensorflow
PYTHONPATH=ml/src .venv/bin/python -m qc evaluate --package artifacts/smoke-a
.venv/bin/python scripts/candidate-gates.py artifacts/smoke-b/metadata.json --baseline artifacts/smoke-a/metadata.json
ADMIN_TOKEN="$ADMIN_TOKEN" .venv/bin/python scripts/api-smoke.py --url http://localhost:5050
.venv/bin/python scripts/benchmark.py --url http://localhost:5050 --requests 20 --workers 4
```

Integration tests require a running API and emulator, create new unique versions and inspections, and do not reset records. Default fixture is an actual TensorFlow-trained ONNX package, not canned predictions. The undertrained fixture's actual metrics fail class recall and baseline regression checks; no negative evaluation is invented in the final test script. Early development runs used explicitly marked tampered negative-test metadata to exercise rejection; those historical local test records are not genuine evaluation results and are not part of the final fixture. Supply `--source artifacts/smoke-a --second-source artifacts/smoke-b` to check distinct independently trained versions. The version IDs are aliases for the source's actual weight/checkpoint metadata; only version name changes.

Local API integration run: 33 images, inference mean .307032 ms, p95 .469625 ms. Container run: 33 images, mean .304833 ms, p95 .376500 ms. Separate 20-request/4-worker native benchmark: .1988245 seconds, 100.59 requests/sec, end-to-end mean 37.46 ms, p95 87.22 ms, inference mean .80313 ms. These repeated procedural PNG measurements include local emulator persistence and are neither production benchmarks nor industrial quality claims.

Initial failures and resolutions: Rust decimal/borrow compile errors; macOS ONNX environment teardown abort fixed with ort rc.11 + Runtime 1.23.2; nested Firestore arrays wrapped under reserved map fields; Vite import.meta.env alias fixed after browser testing; frontend container TypeScript unused callback argument fixed. Default ports are occupied by unrelated projects, so native/browser verification used :5051/:5175 and container :5052. Do not terminate those projects to run tests.

Coverage limits: exact pixel parity is tested on a lossless PNG, not every JPEG decoder/color profile; no near-duplicate/source-group leakage detection; no cloud integration; no IAM browser auth flow; no multi-instance coordination; no sustained industrial load/failure-injection campaign; no complete browser chooser upload due tool failure; CI not remotely executed. Empty datasets are not reported as accuracy; reviewed agreement is selected-label agreement, not population quality.

Screenshots: overview.jpg, model-lab.jpg, review.jpg show actual local stored smoke data. No production data or decorative dataset evidence is fabricated.
