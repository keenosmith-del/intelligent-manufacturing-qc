# Data management

The backend's focused Firestore REST adapter maps typed JSON to native Firestore map/array/integer/double/string/bool/null values under each document's `data` field. `list` consumes nextPageToken; API filtering/pagination currently occurs in memory (V1 benchmark scale, not a large production database query strategy).

| Collection | Key | Stored data |
|---|---|---|
| inspections | UUID | Image reference, immutable prediction/probabilities/version/provenance, timings, timestamp, latest review |
| inspection_reviews | UUID | Inspection ID, reviewed label, note, reviewer, timestamp; append-only actions |
| model_versions | safe version ID | Full package metadata, candidate/validated/rejected state, gate result |
| model_evaluations | version | Held-out metrics, confusion/error/calibration analysis |
| training_runs | version | Config, epoch histories, dataset hash, timestamps |
| model_releases | UUID | Validation, activation or rollback with reasons, target/previous model and timestamp |
| system | active | Persisted active/previous versions and activation timestamp |
| monitoring_snapshots | UUID | Real aggregate at capture time |
| audit_events | UUID | Registration, review and release actions |

Review document, inspection latest-review pointer and audit event commit atomically. Release pointer, history and audit commit atomically. Versions cannot be overwritten through registration. Predictions retain model and dataset hashes for traceability.

Object keys: `images/<uuid>.<png|jpg|bmp>` and `models/<version>/<package-file>`. No image/model binary is placed in Firestore. Registration imports local packages into the chosen object store; cloud mode copies packages to GCS and loads registered weights from GCS at activation/startup. A Cloud Run import workflow can mount a pre-provisioned package directory. Package manifests retain dataset paths; avoid sensitive directories in cloud exports.

Local storage writes temporary files then renames. Inspection storage is compensated with deletion if Firestore persistence fails. A timeout after a Firestore commit can have an ambiguous outcome; operators should inspect records before retrying. No exactly-once submission guarantee or automated orphan reconciliation exists. Disk/Firestore failures are explicit. Keep emulator and local storage resets coordinated. Raw datasets, generated artifacts and uploaded files are ignored by Git; the small project-owned trained CI fixture is intentionally retained.

The Firestore emulator is unauthenticated and normally memory-only. Records persist across page refresh and API restart while the emulator remains running; stopping/recreating it loses database state. Export/import with gcloud tooling when durable emulator snapshots are required. To reset this project's emulator only:

```sh
curl -X DELETE http://localhost:8080/emulator/v1/projects/manufacturing-qc-local/databases/\(default\)/documents
```

This is destructive; not part of startup or automated tests. Integration tests create unique versions/IDs without resetting other records.

Cloud Firestore uses Cloud Run metadata access tokens and application service account IAM. A short-lived `GOOGLE_OAUTH_ACCESS_TOKEN` is supported for deliberate local cloud checks, never committed. Emulator results do not verify production IAM, quotas, indexes or security rules. Uploaded images may contain proprietary production information: implement an organization-approved retention policy, bucket lifecycle rules, audit access and controlled deletion before real deployment. V1 does not offer deletion UI or automatic retention.
