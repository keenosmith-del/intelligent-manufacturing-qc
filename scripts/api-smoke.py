"""Real API/Firestore/ONNX lifecycle integration. Uses fresh version IDs; no DB reset."""

import argparse
import json
import os
import shutil
import urllib.error
import urllib.request
import uuid
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--url", default="http://localhost:5050")
parser.add_argument("--artifacts", default="artifacts")
parser.add_argument("--source", default="tests/fixtures/trained")
parser.add_argument("--second-source", default=None)
parser.add_argument("--failing-source", default="tests/fixtures/undertrained")
args = parser.parse_args()
token = os.environ["ADMIN_TOKEN"]


def call(
    path, body=None, method="GET", authorized=True, content_type="application/json"
):
    headers = {"Content-Type": content_type}
    if authorized:
        headers["Authorization"] = "Bearer " + token
    request = urllib.request.Request(
        args.url + path, data=body, headers=headers, method=method
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        raw = response.read()
        return (
            json.loads(raw)
            if "json" in response.headers.get("Content-Type", "")
            else raw
        )


def post(path, body=None, authorized=True):
    return call(
        path,
        json.dumps(body).encode() if body is not None else None,
        "POST",
        authorized,
    )


def expect_error(code, function):
    try:
        function()
    except urllib.error.HTTPError as error:
        assert error.code == code, (error.code, error.read())
    else:
        raise AssertionError(f"Expected HTTP {code}")


run = "ci-" + uuid.uuid4().hex[:12]
versions = [run + "-a", run + "-b", run + "-bad"]
for index, version in enumerate(versions):
    source = Path(
        args.failing_source
        if index == 2
        else args.second_source
        if index == 1 and args.second_source
        else args.source
    )
    target = Path(args.artifacts) / version
    shutil.copytree(source, target)
    metadata = json.loads((target / "metadata.json").read_text())
    metadata["version"] = version
    (target / "metadata.json").write_text(json.dumps(metadata))
expect_error(
    401, lambda: post("/api/models", {"version": versions[0]}, authorized=False)
)
for version in versions:
    result = post("/api/models", {"version": version})
    assert result["status"] == "candidate"
result = post(f"/api/models/{versions[0]}/activate")
assert result["version"] == versions[0]
assert call("/ready")["model_version"] == versions[0]


def upload(data=None):
    boundary = "qc-boundary-" + uuid.uuid4().hex
    image = (
        data
        if data is not None
        else (Path(args.source) / "parity-input.png").read_bytes()
    )
    body = (
        (
            f'--{boundary}\r\nContent-Disposition: form-data; name="image"; filename="surface.png"\r\n'
            "Content-Type: image/png\r\n\r\n"
        ).encode()
        + image
        + f"\r\n--{boundary}--\r\n".encode()
    )
    return call(
        "/api/inspections",
        body,
        "POST",
        content_type=f"multipart/form-data; boundary={boundary}",
    )


first = upload()
assert first["prediction"]["model_version"] == versions[0]
assert first["prediction"]["synthetic"]
assert len(first["prediction"]["probabilities"]) == 6
assert abs(sum(first["prediction"]["probabilities"]) - 1) < 1e-4
record_id = first["id"]
assert call(f"/api/inspections/{record_id}")["prediction"] == first["prediction"]
assert (
    call(f"/api/inspections/{record_id}/image")
    == (Path(args.source) / "parity-input.png").read_bytes()
)
corrected = "scratches" if first["prediction"]["label"] != "scratches" else "crazing"
reviewed = post(
    f"/api/inspections/{record_id}/reviews",
    {"label": corrected, "note": "Integration review", "reviewer": "CI"},
)
assert reviewed["prediction"] == first["prediction"]
assert reviewed["review"]["label"] == corrected
assert call(f"/api/inspections/{record_id}")["review_status"] == "reviewed"
assert len(call(f"/api/inspections/{record_id}/reviews")) == 1
expect_error(
    400,
    lambda: post(
        f"/api/inspections/{record_id}/reviews",
        {"label": "normal", "note": "", "reviewer": "CI"},
    ),
)
expect_error(400, lambda: upload(b"corrupt image"))
expect_error(404, lambda: call("/api/inspections/missing-record"))
second = post(f"/api/models/{versions[1]}/activate")
assert second["previous"] == versions[0]
assert upload()["prediction"]["model_version"] == versions[1]
rejected = post(f"/api/models/{versions[2]}/validate")
assert rejected["status"] == "rejected" and rejected["gates"]["reasons"]
expect_error(400, lambda: post(f"/api/models/{versions[2]}/activate"))
assert call("/ready")["model_version"] == versions[1]
assert post("/api/models/rollback")["version"] == versions[0]
assert upload()["prediction"]["model_version"] == versions[0]
for _ in range(30):
    upload()
metrics = call("/api/monitoring")
assert (
    metrics["total"] >= 33 and metrics["reviewed"] >= 1 and metrics["corrections"] >= 1
)
assert metrics["active_model_metrics"]["psi"] is not None
assert metrics["process_telemetry"]["failures"] >= 1
snapshot = post("/api/monitoring/snapshots")
assert snapshot["metrics"]["total"] == metrics["total"]
assert any(s["id"] == snapshot["id"] for s in call("/api/monitoring/snapshots"))
assert any(
    r["action"] == "rollback" and r["version"] == versions[0]
    for r in call("/api/releases")
)
assert call("/api/inspections?review_status=reviewed&limit=1")["items"]
print(
    json.dumps(
        {
            "passed": True,
            "versions": versions,
            "inspection_id": record_id,
            "inspections": metrics["total"],
            "mean_inference_ms": metrics["mean_latency_ms"],
            "p95_inference_ms": metrics["p95_latency_ms"],
            "active": versions[0],
        },
        indent=2,
    )
)
