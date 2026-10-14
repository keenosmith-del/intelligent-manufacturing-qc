"""Actual TensorFlow training, held-out evaluation, ONNX export and numerical parity."""

import hashlib
import json
import re
import time
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

from .data import CLASSES, digest, load_split
from .evaluation import evaluate


def now():
    return datetime.now(timezone.utc).isoformat()


def train(manifest_path, config, output, version):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}", version):
        raise ValueError("invalid model version")
    import tensorflow as tf
    import onnx
    import onnxruntime as ort
    import tf2onnx

    if config["epochs"] < 1 or config["batch_size"] < 1:
        raise ValueError("epochs and batch size must be positive")
    tf.config.threading.set_inter_op_parallelism_threads(1)
    tf.config.threading.set_intra_op_parallelism_threads(1)
    tf.keras.utils.set_random_seed(config["seed"])
    tf.config.experimental.enable_op_determinism()
    manifest = json.loads(Path(manifest_path).read_text())
    prep = manifest["preprocessing"]
    if manifest["classes"] != CLASSES or (config["width"], config["height"]) != (
        prep["width"],
        prep["height"],
    ):
        raise ValueError("training config does not match dataset contract")
    out = Path(output) / version
    out.mkdir(parents=True, exist_ok=False)
    started = now()
    x_train, y_train, _ = load_split(manifest, "train")
    x_val, y_val, _ = load_split(manifest, "validation")
    x_test, y_test, rows = load_split(manifest, "test")
    model = tf.keras.Sequential(
        [
            tf.keras.layers.Input((prep["height"], prep["width"], 1), name="images"),
            tf.keras.layers.RandomFlip("horizontal_and_vertical", seed=config["seed"]),
            tf.keras.layers.RandomRotation(0.05, seed=config["seed"]),
            tf.keras.layers.Conv2D(16, 3, activation="relu", padding="same"),
            tf.keras.layers.MaxPooling2D(),
            tf.keras.layers.Conv2D(32, 3, activation="relu", padding="same"),
            tf.keras.layers.MaxPooling2D(),
            tf.keras.layers.Conv2D(64, 3, activation="relu", padding="same"),
            tf.keras.layers.GlobalAveragePooling2D(),
            tf.keras.layers.Dense(32, activation="relu"),
            tf.keras.layers.Dense(6, activation="softmax", name="probabilities"),
        ],
        name="surface_cnn",
    )
    model.compile(
        optimizer=tf.keras.optimizers.Adam(config["learning_rate"]),
        loss="sparse_categorical_crossentropy",
        metrics=["accuracy"],
    )
    history = model.fit(
        x_train,
        y_train,
        validation_data=(x_val, y_val),
        epochs=config["epochs"],
        batch_size=config["batch_size"],
        callbacks=[
            tf.keras.callbacks.EarlyStopping(
                monitor="val_loss", patience=config["patience"], restore_best_weights=True
            ),
            tf.keras.callbacks.ModelCheckpoint(
                str(out / "best.keras"), monitor="val_loss", save_best_only=True
            ),
        ],
        verbose=2,
    )
    # Evaluate the saved best checkpoint, not an arbitrary final epoch.
    model = tf.keras.models.load_model(out / "best.keras")
    validation_loss, validation_accuracy = model.evaluate(x_val, y_val, verbose=0)
    start = time.perf_counter()
    tf_prob = model(x_test, training=False).numpy()
    tf_latency = (time.perf_counter() - start) * 1000 / len(x_test)
    metrics = evaluate(y_test, tf_prob, rows)
    metrics.update(
        validation_loss=float(validation_loss), validation_accuracy=float(validation_accuracy)
    )
    spec = (tf.TensorSpec((None, prep["height"], prep["width"], 1), tf.float32, name="images"),)

    @tf.function(input_signature=spec)
    def serving(images):
        return {"probabilities": model(images, training=False)}

    tf2onnx.convert.from_function(
        serving, input_signature=spec, opset=17, output_path=str(out / "model.onnx")
    )
    graph = onnx.load(out / "model.onnx")
    onnx.checker.check_model(graph)
    options = ort.SessionOptions()
    options.intra_op_num_threads = 1
    session = ort.InferenceSession(
        str(out / "model.onnx"), options, providers=["CPUExecutionProvider"]
    )
    if session.get_inputs()[0].shape[1:] != [prep["height"], prep["width"], 1]:
        raise ValueError("ONNX input contract mismatch")
    onnx_prob = session.run(None, {session.get_inputs()[0].name: x_test})[0]
    delta = float(np.max(np.abs(tf_prob - onnx_prob)))
    parity = {
        "max_absolute_error": delta,
        "tolerance": 1e-5,
        "same_classes": bool(np.array_equal(tf_prob.argmax(1), onnx_prob.argmax(1))),
        "sample_count": len(x_test),
        "passed": delta <= 1e-5,
        "tensorflow": tf.__version__,
        "tf2onnx": tf2onnx.__version__,
        "onnxruntime": ort.__version__,
        "onnx": onnx.__version__,
        "opset": 17,
    }
    parity["passed"] = parity["passed"] and parity["same_classes"]
    if not parity["passed"]:
        raise ValueError(f"ONNX parity failed: {parity}")
    # Batch=1 warm latency, measured across the actual held-out samples.
    session.run(None, {session.get_inputs()[0].name: x_test[:1]})
    timings = []
    for sample in x_test:
        start = time.perf_counter()
        session.run(None, {session.get_inputs()[0].name: sample[None]})
        timings.append((time.perf_counter() - start) * 1000)
    metrics.update(
        onnx_latency_ms=float(np.mean(timings)),
        onnx_p95_ms=float(np.percentile(timings, 95)),
        tensorflow_latency_ms=tf_latency,
    )
    test_hash = hashlib.sha256(
        json.dumps(sorted((r["sha256"], r["class_index"]) for r in rows)).encode()
    ).hexdigest()
    metadata = {
        "schema_version": 1,
        "version": version,
        "classes": CLASSES,
        "preprocessing": prep,
        "dataset_version": manifest["dataset_version"],
        "test_split_hash": test_hash,
        "provenance": manifest["provenance"],
        "framework": "TensorFlow " + tf.__version__,
        "architecture": "surface_cnn",
        "training_config": config,
        "started_at": started,
        "finished_at": now(),
        "history": history.history,
        "evaluation": metrics,
        "parity": parity,
        "sha256": digest(out / "model.onnx"),
        "artifact_bytes": (out / "model.onnx").stat().st_size,
        "reference_distribution": (
            np.bincount(tf_prob.argmax(1), minlength=6) / len(y_test)
        ).tolist(),
    }
    for name, data in [
        ("metadata.json", metadata),
        ("evaluation.json", metrics),
        ("classes.json", CLASSES),
        ("preprocessing.json", prep),
        ("parity.json", parity),
        ("manifest.json", manifest),
        ("training-config.json", config),
    ]:
        (out / name).write_text(json.dumps(data, indent=2))
    # Shared fixture compares Python preprocessing AND probabilities with Rust.
    fixture_image = Path(manifest["root"]) / rows[0]["path"]
    from PIL import Image

    with Image.open(fixture_image) as img:
        img.convert("RGB").save(out / "parity-input.png")
    (out / "rust-parity.json").write_text(
        json.dumps(
            {"pixels": x_test[0].reshape(-1).tolist(), "probabilities": onnx_prob[0].tolist()}
        )
    )
    print(
        json.dumps(
            {
                "artifact": str(out),
                "synthetic": manifest["provenance"]["synthetic"],
                "macro_f1": metrics["macro_f1"],
                "parity": parity,
            },
            indent=2,
        )
    )
    return metadata
