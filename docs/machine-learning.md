# Machine learning

NEU-CLS is the intended real dataset. The [official NEU source](https://faculty.neu.edu.cn/songkc/en/zdylm/263270/list/index.htm) describes 1,800 grayscale 200×200 images, six classes of 300 images, and offers image-only downloads. Attribute Song and Yan, *A noise robust method based on completed local binary patterns for hot-rolled steel strip surface defects*, Applied Surface Science 285 (2013), 858–864. The page does not supply explicit redistribution terms. Obtain permission/appropriate usage rights before using downloaded images; none are bundled or automatically downloaded here.

Place licensed images under `data/neu/<class>/*.{png,jpg,bmp}`. Supported class names: crazing, inclusion, patches, pitted_surface, rolled_in_scale, scratches; NEU filename prefixes Cr/In/Pa/PS/RS/Sc are also accepted. Provide a provenance JSON with source, license, dataset_id and synthetic=false. The smoke generator owns its procedural images and records synthetic=true. Smoke metrics test software plumbing only.

The fixed mapping is ordered as above. Preprocessing uses deterministic nearest-neighbor integer coordinate sampling and an integer grayscale formula shared by Python and Rust; normalization divides by 255, NHWC float32 [N,H,W,1]. This deliberately avoids library-specific resize and grayscale rounding differences. Training-only flips/rotation follow preprocessing. Final evaluation uses a held-out test split; validation chooses the checkpoint. Exact content hashes and decoded-pixel hashes remove duplicates before per-class seeded 70/15/15 splits.

## Training and evaluation

Architecture: Conv2D(16,3×3,relu,same) → 2×2 max-pool → Conv2D(32) → max-pool → Conv2D(64) → global average pool → Dense(32,relu) → Dense(6,softmax). Training-only Keras RandomFlip and RandomRotation(.05) precede convolutions; they are inactive during export/inference. Adam learning rate .001; sparse categorical cross-entropy; batch 32; seed 42; maximum 30 epochs in baseline configuration (smoke verification explicitly used 35); patience 5 on validation loss. Best saved `.keras` checkpoint is restored for export and test evaluation.

Default preparation uses per-class 70% train, 15% validation, remainder test with at least 7 unique images/class. Count rounding is recorded in manifest. Fixed-seed ordering is deterministic; file hash validation refuses changed images during loading. No label tuning against final test data is automated. Different training seeds preserve dataset/split version so evaluation remains comparable.

```sh
PYTHONPATH=ml/src .venv/bin/python -m qc synthetic --count 36
PYTHONPATH=ml/src .venv/bin/python -m qc prepare --data data/synthetic --provenance data/synthetic/provenance.json
PYTHONPATH=ml/src .venv/bin/python -m qc train --version smoke-a --epochs 35
PYTHONPATH=ml/src .venv/bin/python -m qc train --version smoke-b --config ml/configs/smoke-b.json --epochs 35
PYTHONPATH=ml/src .venv/bin/python -m qc evaluate --package artifacts/smoke-a --output artifacts/reevaluation.json
.venv/bin/python scripts/validate-artifact.py artifacts/smoke-a --tensorflow
```

Exports use a concrete TensorFlow serving function with augmentation disabled, float32 NHWC input and ONNX opset 17. ONNX checker and Runtime execute the test batch, assert dimensions, finite normalized probabilities and matching argmax classes with max absolute error <=1e-5. Report records tool versions, graph checksum, model bytes and warm batch-1 latency distribution. Python/Rust share a generated PNG fixture with exact preprocessed floats and real probabilities. JPEG decoder implementations may differ slightly across libraries; lossless PNG is the parity reference. Verify representative production formats before deployment.

## Actual smoke results (2026-10-14)

216 procedural images: 150 train / 30 validation / 36 test. This synthetic task is deliberately easy and includes class-correlated patterns/brightness. The results do not estimate NEU accuracy or industrial usefulness.

| Version | Seed | Max / actual epochs | Test accuracy | Macro-F1 | TF/ONNX max error at export |
|---|---|---|---|---|---|
| smoke-v1 | 42 | 12 / 12 | .666667 | .555556 | 1.788139e-7 |
| smoke-a | 42 | 35 / 25 | 1.0 | 1.0 | 3.576279e-7 |
| smoke-b | 43 | 35 / 32 | 1.0 | 1.0 | 4.768372e-7 |

Best validation checkpoints: smoke-a epoch 20; smoke-b epoch 27. All 36 test argmax labels matched at export with ONNX Runtime 1.22.0. The runtime was subsequently upgraded to 1.23.2 with ort rc.11 to resolve macOS teardown. Independent full TensorFlow parity revalidation of smoke-a on 1.23.2 passed, max error 4.768372e-7. Rust shared-fixture parity also passed on 1.23.2. Original reports retain original exporter version for honest provenance.

Packages record held-out precision/recall/F1 per class, weighted-F1, confusion matrix, validation loss, epoch curves, calibration ECE/bins, high-confidence mistakes and low-confidence examples. Confidence is not a calibrated production probability simply because softmax is used. No normal class, anomaly detection, bounding boxes or real industrial evaluation exists. Dataset quality, subtle duplicates/near duplicates, source-group leakage, calibration, robustness and production throughput require real-data work. Exact pixel deduplication does not detect near duplicates or shared production batches.
