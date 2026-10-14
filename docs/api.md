# REST API

Base URL `http://localhost:5050`. JSON errors use `{ "error": { "message": "..." } }`; framework malformed JSON/body-limit errors may be plain text. 400 invalid request, 401 missing operator token, 404 missing record, 503 dependency/model unavailable. CORS allows only `CORS_ORIGIN`. Max HTTP body 11 MiB; image 10 MiB; decoded pixels <=16 million. PNG/JPEG/BMP only. Model IDs: 1..64 ASCII letters/digits/`.`/`_`/`-`, excluding `.` and `..`.

| Method | Route | Contract |
|---|---|---|
| GET | `/health` | Process liveness |
| GET | `/ready` | Firestore reachable and active session successfully executes a zero tensor; version/load time |
| GET | `/api/classes` | Fixed ordered class strings |
| POST | `/api/inspections` | Multipart field `image`; returns persisted Inspection |
| GET | `/api/inspections` | `{items,total,page,limit}`; query search, category, review_status, max_confidence (0..1), page, limit (1..100), sort (newest/oldest) |
| GET | `/api/inspections/{id}` | Inspection with immutable prediction and latest review |
| GET | `/api/inspections/{id}/image` | Original image from controlled object key |
| GET | `/api/inspections/{id}/reviews` | All immutable review actions, newest first |
| POST | `/api/inspections/{id}/reviews` | Operator; `{label,note,reviewer}`; returns updated Inspection |
| GET | `/api/overview` | Aggregate monitoring metrics + recent inspections |
| GET | `/api/models` | `{items:[ModelRecord],active:ActiveModel|null}` |
| POST | `/api/models` | Operator; `{version}` imports checked local package into configured object store and Firestore |
| POST | `/api/models/{version}/validate` | Operator; executes artifact check and quality gates; returns record with reasons |
| POST | `/api/models/{version}/activate` | Operator; revalidates, commits release, swaps executable session |
| POST | `/api/models/rollback` | Operator; loads previous validated version and commits rollback |
| GET | `/api/training-runs` | Recorded configurations, epoch histories and timestamps |
| GET | `/api/releases` | Validation/activation/rollback events, newest first |
| GET | `/api/monitoring` | Real aggregates, model-scoped drift, process counters |
| GET | `/api/monitoring/snapshots` | Persisted measurement snapshots |
| POST | `/api/monitoring/snapshots` | Operator; capture real current aggregate |

Operator mutations require `Authorization: Bearer $ADMIN_TOKEN`. Registration and review are also protected. Uploads and reads assume trusted local access. Cloud Run must require IAM authentication at the service boundary; this token alone is not production user/role management.

Review label must be one of `/api/classes`; reviewer 1..100 characters, note <=2000 bytes. Original `prediction` remains unchanged; `review` and `review_status` reflect latest action. Each action is stored separately.

```sh
curl -F image=@data/synthetic/crazing/000.png http://localhost:5050/api/inspections
curl -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"version":"smoke-a"}' http://localhost:5050/api/models
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" http://localhost:5050/api/models/smoke-a/activate
curl -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"label":"crazing","note":"Verified surface pattern","reviewer":"Local operator"}' \
  http://localhost:5050/api/inspections/INSPECTION_ID/reviews
```

Inspection response: id, image_key (safe relative object reference), media_type, created_at, status, review_status, review|null, total_ms, prediction {label,confidence,probabilities[6],model_version,dataset_version,synthetic,inference_ms}. ModelRecord: metadata (training/export package), status (candidate/validated/rejected), gates|null. ActiveModel: version, previous|null, activated_at. Full package schemas are defined in backend/src/models.rs and frontend/src/types/index.ts.
