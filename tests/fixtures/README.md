# Executable CI fixture

`trained/` contains the actual compact TensorFlow CNN exported during smoke-a training on 216 project-owned procedural images. The ONNX binary is approximately 102 KiB, intentionally small enough to audit and retain as an executable CI fixture. No third-party dataset images are included. `parity-input.png` is one procedural held-out sample; `rust-parity.json` is its Python preprocessed tensor and actual ONNX probabilities. Metadata/history/evaluation correspond to the real training run; all quality metrics are **synthetic smoke only**.

Dataset manifest root is rewritten as `data/synthetic` for portability; recreate it with the documented generator (36 images/class, seed 42). Recompute package evaluation if changing any image or preprocessing. Training used TensorFlow 2.16.2, Keras 3.15.1, tf2onnx 1.16.1, ONNX opset 17. Export originally ran ONNX Runtime 1.22.0; Rust runtime 1.23.2 parity is verified separately. This fixture is not eligible for cloud activation.

`undertrained/` is the actual smoke-v1 checkpoint exported after 12 epochs, with held-out macro-F1 .555556, accuracy .666667, and some zero-recall classes. It is used for negative release tests without inventing evaluation metrics. Both binaries total about 208 KiB. Copies made by API tests change only the unique model version identifier, never weights or reported evaluation.
