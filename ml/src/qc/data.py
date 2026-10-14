"""Dataset validation and the pixel contract shared with Rust."""

import hashlib
import json
from collections import Counter
from pathlib import Path

import numpy as np
from PIL import Image

CLASSES = ["crazing", "inclusion", "patches", "pitted_surface", "rolled_in_scale", "scratches"]
ALIASES = dict(zip(["cr", "in", "pa", "ps", "rs", "sc"], CLASSES))


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def pixels(image, width, height):
    """Exact integer nearest-neighbor and Rec.601-like grayscale; no platform resize."""
    rgb = np.asarray(image.convert("RGB"), dtype=np.uint32)
    ys = np.arange(height) * rgb.shape[0] // height
    xs = np.arange(width) * rgb.shape[1] // width
    rgb = rgb[ys[:, None], xs[None, :]]
    gray = (299 * rgb[..., 0] + 587 * rgb[..., 1] + 114 * rgb[..., 2] + 500) // 1000
    return (gray.astype(np.float32) / np.float32(255))[..., None]


def label_for(path):
    for value in (path.parent.name, path.stem.split("_")[0]):
        value = value.lower().replace("-", "_").replace(" ", "_")
        if value in CLASSES:
            return value
        if value in ALIASES:
            return ALIASES[value]
    raise ValueError("unknown class")


def prepare(root, output, provenance, seed=42, width=64, height=64):
    root, output = Path(root).resolve(), Path(output)
    if not 8 <= width <= 512 or not 8 <= height <= 512:
        raise ValueError("dimensions must be 8..512")
    for key in ("dataset_id", "source", "license", "synthetic"):
        if key not in provenance:
            raise ValueError(f"missing provenance: {key}")
    if not isinstance(provenance["synthetic"], bool):
        raise ValueError("synthetic must be boolean")
    records, rejected, duplicates, seen = [], [], [], {}
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.name == "provenance.json":
            continue
        try:
            if path.suffix.lower() not in {".jpg", ".jpeg", ".png", ".bmp"}:
                raise ValueError("unsupported format")
            label = label_for(path)
            with Image.open(path) as image:
                image.load()
                if image.width * image.height > 16_000_000:
                    raise ValueError("image exceeds 16 megapixels")
                rgb = image.convert("RGB")
                pixel_hash = hashlib.sha256(str(rgb.size).encode() + rgb.tobytes()).hexdigest()
            if pixel_hash in seen:
                if seen[pixel_hash] != label:
                    raise ValueError("duplicate image has conflicting labels")
                duplicates.append(str(path.relative_to(root)))
                continue
            seen[pixel_hash] = label
            records.append(
                {
                    "path": str(path.relative_to(root)),
                    "label": label,
                    "class_index": CLASSES.index(label),
                    "sha256": digest(path),
                    "pixel_sha256": pixel_hash,
                }
            )
        except (ValueError, OSError) as error:
            rejected.append({"path": str(path.relative_to(root)), "reason": str(error)})
    rng = np.random.default_rng(seed)
    for label in CLASSES:
        group = [r for r in records if r["label"] == label]
        if len(group) < 7:
            raise ValueError(f"{label}: at least 7 unique readable images required")
        rng.shuffle(group)
        n_train, n_val = int(len(group) * 0.7), max(1, int(len(group) * 0.15))
        for i, row in enumerate(group):
            row["split"] = (
                "train" if i < n_train else "validation" if i < n_train + n_val else "test"
            )
    version = hashlib.sha256(
        json.dumps(
            {
                "records": records,
                "provenance": provenance,
                "seed": seed,
                "width": width,
                "height": height,
            },
            sort_keys=True,
        ).encode()
    ).hexdigest()
    manifest = {
        "dataset_version": version,
        "provenance": provenance,
        "root": str(root),
        "classes": CLASSES,
        "seed": seed,
        "records": records,
        "preprocessing": {
            "width": width,
            "height": height,
            "channels": 1,
            "layout": "NHWC",
            "resize": "integer_nearest",
            "grayscale": "integer_601",
            "normalization": "divide_255",
        },
        "rejected": rejected,
        "duplicates": duplicates,
        "class_counts": dict(Counter(r["label"] for r in records)),
        "split_counts": {
            s: dict(Counter(r["label"] for r in records if r["split"] == s))
            for s in ("train", "validation", "test")
        },
    }
    output.mkdir(parents=True, exist_ok=True)
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2))
    (output / "validation.md").write_text(
        f"# Dataset validation\n\nVersion: {version}\n\nSource: {provenance['source']}\n"
        f"\nSynthetic: {provenance['synthetic']}\n\nUnique images: {len(records)}\n"
        f"\nRejected: {len(rejected)}; duplicates: {len(duplicates)}\n\n"
        f"```json\n{json.dumps(manifest['split_counts'], indent=2)}\n```\n"
        f"\nRejected files:\n```json\n{json.dumps(rejected, indent=2)}\n```\n"
    )
    return manifest


def load_split(manifest, split):
    rows = [r for r in manifest["records"] if r["split"] == split]
    tensors = []
    for row in rows:
        path = Path(manifest["root"]) / row["path"]
        if digest(path) != row["sha256"]:
            raise ValueError(f"dataset changed after preparation: {row['path']}")
        with Image.open(path) as image:
            tensors.append(
                pixels(
                    image, manifest["preprocessing"]["width"], manifest["preprocessing"]["height"]
                )
            )
    return np.stack(tensors), np.asarray([r["class_index"] for r in rows]), rows


def synthetic(root, count=24, seed=42):
    """Procedural patterns, owned smoke fixtures; no industrial validity."""
    root = Path(root)
    rng = np.random.default_rng(seed)
    for index, label in enumerate(CLASSES):
        folder = root / label
        folder.mkdir(parents=True, exist_ok=True)
        for n in range(count):
            yy, xx = np.mgrid[:64, :64]
            noise = rng.normal(0, 15, (64, 64))
            # Each image varies continuously; pattern identity only supports plumbing tests.
            angle = rng.uniform(-0.25, 0.25)
            coordinate = xx * np.cos(angle) + yy * np.sin(angle)
            base = 40 + index * 30 + noise
            if index == 0:
                base += 60 * (np.sin(coordinate / 3) > 0.7)
            elif index == 1:
                base += 50 * ((xx - rng.integers(20, 44)) ** 2 + (yy - 32) ** 2 < 100)
            elif index == 2:
                base[10:40, 12:50] += 50
            elif index == 3:
                base += rng.integers(0, 2, (64, 64)) * 45
            elif index == 4:
                base += 40 * (np.sin(xx / 5) * np.cos(yy / 5))
            else:
                base += 45 * (np.sin(coordinate / 2) > 0.5)
            Image.fromarray(np.clip(base, 0, 255).astype(np.uint8)).save(folder / f"{n:03}.png")
    provenance = {
        "dataset_id": "synthetic-smoke-v1",
        "source": "project procedural generator",
        "license": "project-owned smoke fixtures",
        "synthetic": True,
    }
    (root / "provenance.json").write_text(json.dumps(provenance, indent=2))
    return provenance
