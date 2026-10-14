# Containers and optional cloud deployment

## Local Compose

```sh
export ADMIN_TOKEN='replace-with-a-long-local-operator-token'
docker compose up --build -d
```

Frontend :5173, backend :5050, emulator :8080, all published to loopback. The API image uses Debian bookworm, pinned Rust build dependencies, packaged ONNX Runtime 1.23.2, CA certificates and libgomp, and runs as UID 10001. Readiness executes the actual active session and checks Firestore. Backend may be healthy at process level but readiness unhealthy until a model is registered/activated. Frontend is a Vite production build served by Nginx with SPA fallback.

Compose defaults to Firestore project `manufacturing-qc-compose` and named `qc-storage` volume, separate from direct local development metadata/files. Native development uses `manufacturing-qc-local` and `.storage`. Keep the namespace and object storage together when switching modes. Raw artifacts are mounted read-only for privileged import; uploaded/registered objects are copied to the writable named volume. Emulator contents are memory-only; named image storage alone does not preserve Firestore records when the emulator restarts.

Train outside the web containers, then enter operator token in Model Lab and register/activate your version from the mounted artifacts directory. Only `ALLOW_SYNTHETIC` in emulator mode permits smoke models. `docker compose stop` preserves containers/volumes but emulator process stop loses in-memory database state. Avoid `down -v` unless you intend to delete local object storage.

Backend and frontend image builds and actual non-root backend container inference/persistence/release workflows were executed locally. Default host ports are occupied by other projects on this Mac; full Compose verification used API :5052 and frontend :5176 with a separate Firestore namespace. Startup, SPA serving, real inference/review/release flows and active-pointer restore after API-container restart were checked. To reproduce free-port Compose startup: `API_PORT=5052 FRONTEND_PORT=5176 ADMIN_TOKEN="$ADMIN_TOKEN" docker compose up --build -d`. Default-port bindings were not exercised against occupied ports.

## Cloud readiness (not deployed or verified)

Provision explicitly: Firestore Native database; private GCS bucket; Artifact Registry repository; separate deployment/runtime service accounts; Secret Manager `qc-operator-token`; GitHub WIF/OIDC provider scoped to the repository and approved environment. Runtime IAM needs Firestore data access and object read/write/delete on this bucket only; Secret Manager access on the named secret. Deployer needs narrowly scoped Artifact Registry publishing, Cloud Run deployment and service-account impersonation; use organization policy to restrict public exposure. Do not put service-account JSON keys in GitHub secrets.

`deploy-gcp.yml` is manual-only, skips when basic configuration is absent and uses `gcp-production` environment. Configure GitHub required reviewers on that environment. Variables: GCP_PROJECT_ID, WIF_PROVIDER, DEPLOY_SERVICE_ACCOUNT, GCP_REGION, ARTIFACT_REPOSITORY, CLOUD_RUN_SERVICE, RUNTIME_SERVICE_ACCOUNT, GCS_BUCKET, CORS_ORIGIN. The workflow builds an immutable SHA-tagged backend image and deploys a private, IAM-authenticated Cloud Run service with max instances 1. It does not provision billable infrastructure. The Docker container serves the main Rust API; frontend hosting remains an explicitly configured static hosting/container concern.

Cloud mode uses metadata access tokens, GCS object storage and production Firestore. Uploaded images and model binaries are never Firestore documents. Before deploying, register a rights-verified real model in the project's registry and bucket; active pointers/artifacts must use the production namespace. For registration in Cloud Run, mount a pre-provisioned GCS import directory or import with a local authenticated single-operator API against production storage before service startup. The workflow does not automatically import/promote a model or bypass its gates.

The frontend does not implement Google IAM/OIDC browser sign-in. For private cloud review, use an authenticated organization access gateway or `gcloud run services proxy` with an authorized operator; configure matching CORS/build-time API URL. V1 is a local single-operator product and is not claimed production-secure. A bearer operator token is not a complete identity/role system. Single-instance release/review coordination does not provide distributed synchronization during overlapping Cloud Run revisions; freeze release actions during software rollouts. Horizontal scaling requires a transactional distributed coordinator.

Cloud verification still required: IAM/WIF policy review, authenticated frontend access, GCS/Firestore production integration, secrets, model import/restoration, readiness, resource limits, network policy, rollout overlap and rollback. No GCP credentials or paid resources were used during implementation.
