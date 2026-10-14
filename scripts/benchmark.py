"""Measured real HTTP inference throughput on a chosen image; creates inspections."""

import argparse
import concurrent.futures
import json
import time
import urllib.request
import uuid
from pathlib import Path

import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--url", default="http://localhost:5050")
parser.add_argument("--image", default="tests/fixtures/trained/parity-input.png")
parser.add_argument("--requests", type=int, default=20)
parser.add_argument("--workers", type=int, default=4)
args = parser.parse_args()
assert 1 <= args.requests <= 1000 and 1 <= args.workers <= 16
image = Path(args.image).read_bytes()


def submit(_):
    boundary = "qc-" + uuid.uuid4().hex
    body = (
        (
            f'--{boundary}\r\nContent-Disposition: form-data; name="image"; filename="benchmark.png"\r\n'
            "Content-Type: image/png\r\n\r\n"
        ).encode()
        + image
        + f"\r\n--{boundary}--\r\n".encode()
    )
    req = urllib.request.Request(
        args.url + "/api/inspections",
        body,
        {"Content-Type": f"multipart/form-data; boundary={boundary}"},
    )
    start = time.perf_counter()
    with urllib.request.urlopen(req, timeout=30) as response:
        result = json.loads(response.read())
    return {
        "end_to_end_ms": (time.perf_counter() - start) * 1000,
        "inference_ms": result["prediction"]["inference_ms"],
        "version": result["prediction"]["model_version"],
    }


start = time.perf_counter()
with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
    rows = list(pool.map(submit, range(args.requests)))
elapsed = time.perf_counter() - start
print(
    json.dumps(
        {
            "requests": len(rows),
            "workers": args.workers,
            "elapsed_seconds": elapsed,
            "throughput_requests_per_second": len(rows) / elapsed,
            "end_to_end_mean_ms": float(np.mean([r["end_to_end_ms"] for r in rows])),
            "end_to_end_p95_ms": float(
                np.percentile([r["end_to_end_ms"] for r in rows], 95)
            ),
            "inference_mean_ms": float(np.mean([r["inference_ms"] for r in rows])),
            "versions": sorted({r["version"] for r in rows}),
            "scope": "local repeated synthetic fixture",
        },
        indent=2,
    )
)
