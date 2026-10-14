# Intelligent Manufacturing Quality Control

**Machine Learning Engineering · MLOps · Rust · TensorFlow · Computer Vision**

An intelligent manufacturing quality-control platform that uses deep learning to classify visual product defects, a Rust-based inference service for prediction delivery, and automated MLOps workflows for model validation and deployment.

The project demonstrates the complete machine learning lifecycle, from dataset preparation and neural network training to inference, monitoring and controlled model releases.

## Overview

Manufacturing quality-control processes depend on accurate and consistent defect detection.

Manual visual inspection can introduce variability, slow down quality reviews and make it difficult to maintain consistent inspection records.

This project explores how a custom-trained computer vision model can support manufacturing inspection workflows while providing engineering controls for monitoring, evaluating and improving deployed models.

Unlike an application that only consumes an external AI API, this platform will include direct model development and training using TensorFlow.

## Business Problem

Manufacturing organizations face challenges involving:

- Inconsistent manual defect classification.
- Delayed identification and investigation of defects.
- Limited visibility into inspection performance.
- Difficulty maintaining traceable inspection histories.
- Model performance degradation as production data changes.
- Risky or inconsistent deployment of updated ML models.

## Proposed Solution

A full-stack machine learning application combining visual inspection, model inference, human review and automated ML delivery.

### Intelligent Visual Inspection

- Upload manufacturing inspection images.
- Classify known defect categories using a trained computer vision model.
- Return prediction confidence and model version.
- Display results through an interactive inspection dashboard.
- Record inspection outcomes for later analysis.
- Allow quality engineers to review and correct predictions.

### Machine Learning Engineering

- Prepare and validate labelled image datasets.
- Train an image classifier using TensorFlow/Keras.
- Evaluate alternative model architectures.
- Track model versions, configurations and evaluation results.
- Export trained models for inference.
- Validate numerical consistency between training and deployed inference.
- Analyze classification failures and model limitations.

### MLOps & Model Delivery

- Automate software and model validation.
- Version model artifacts and evaluation metadata.
- Introduce quality gates before model promotion.
- Deploy containerized inference services.
- Monitor prediction behaviour and inference performance.
- Collect reviewer feedback to support future model improvements.
- Support controlled model rollback.

## Technology Stack

| Layer             | Technologies                                                   |
| ----------------- | -------------------------------------------------------------- |
| Frontend          | React, TypeScript, Tailwind CSS                                |
| Inference Backend | Rust, Axum, Tokio                                              |
| Machine Learning  | Python, TensorFlow, Keras                                      |
| Model Runtime     | ONNX Runtime                                                   |
| Database          | Google Cloud Firestore                                         |
| Object Storage    | Google Cloud Storage                                           |
| Containerization  | Docker                                                         |
| Cloud Deployment  | Google Cloud Run, Artifact Registry                            |
| CI/CD             | GitHub Actions                                                 |
| Monitoring        | Application telemetry, model quality and prediction monitoring |
| Testing           | Python, Rust, API and model integration tests                  |

## Architecture

The proposed architecture separates model training, inference serving, data persistence and release management.

### Model Development

1. Acquire and validate a labelled manufacturing inspection dataset.
2. Prepare image preprocessing and training pipelines.
3. Train a TensorFlow image-classification model.
4. Evaluate classification performance against held-out test data.
5. Export the trained model to ONNX.
6. Validate inference compatibility and numerical parity.
7. Store versioned model artifacts and evaluation metadata.

### Inference Workflow

1. A user uploads an inspection image.
2. The Rust API validates the request and image format.
3. The inference service loads the selected model artifact.
4. Image preprocessing prepares the model input.
5. ONNX Runtime executes inference.
6. The API returns a predicted class, confidence score and model version.
7. Inspection metadata and results are stored in Firestore.
8. Authorized users can review predictions and submit corrections.

Images and model artifacts will be stored in object storage rather than embedded in Firestore records.

## Machine Learning Approach

The initial implementation will focus on supervised image classification using a suitable manufacturing surface-defect dataset.

Potential approaches include:

- Convolutional neural networks.
- Transfer learning using a compatible TensorFlow vision model.
- Data augmentation.
- Stratified training, validation and test splits.
- Per-class error analysis.
- Model calibration and confidence analysis.

A dataset containing only defect categories cannot independently establish reliable defect-free versus defective classification. A production-style pass/fail workflow would require appropriate normal-product examples and further validation.

## Engineering Objectives

This project is intended to demonstrate:

- Practical deep learning model development.
- TensorFlow training and evaluation.
- High-performance Rust backend engineering.
- Model export and cross-runtime inference.
- Cloud-native storage and persistence.
- Automated testing and CI/CD.
- Model validation and controlled releases.
- Monitoring and continuous model improvement.
- Reproducible machine learning workflows.

## Evaluation & Performance

Planned measurements include:

| Metric                          | Purpose                                     |
| ------------------------------- | ------------------------------------------- |
| Per-class precision and recall  | Evaluate defect classification              |
| Macro-F1                        | Measure balanced classification quality     |
| Confusion matrix                | Identify frequently confused defect classes |
| False-negative rates            | Analyze missed defect categories            |
| Inference latency               | Measure prediction response time            |
| Throughput                      | Evaluate Rust serving capacity              |
| Model parity                    | Compare TensorFlow and ONNX predictions     |
| Model version performance       | Track model quality across releases         |
| Prediction distribution changes | Identify potential data or model drift      |

Results will be recorded after training and benchmarking. No performance claims are made before validation.

## CI/CD & MLOps

The planned delivery process separates routine application CI from model-training workflows.

### Application CI

- Run Rust formatting, lint and unit tests.
- Run Python formatting, lint and tests.
- Verify API and inference integrations.
- Build and inspect container images.
- Validate the configured model artifact.

### Model Validation

- Validate datasets and training metadata.
- Evaluate candidate models against predefined quality thresholds.
- Verify TensorFlow-to-ONNX output compatibility.
- Compare candidate performance against the currently approved model.
- Reject model releases that fail required quality gates.

### Deployment

- Publish versioned container images.
- Deploy to a staging environment.
- Execute integration and inference smoke tests.
- Promote approved model and service versions.
- Preserve a rollback path.

Model training will be triggered separately rather than performed automatically on every application code change.

## Development Roadmap

- [ ] Phase 1 — Repository structure, dataset selection and ML environment.
- [ ] Phase 2 — TensorFlow training pipeline and baseline classifier.
- [ ] Phase 3 — Model evaluation, export and reproducibility.
- [ ] Phase 4 — Rust inference API and ONNX integration.
- [ ] Phase 5 — Firestore, Cloud Storage and inspection dashboard.
- [ ] Phase 6 — CI/CD and model release quality gates.
- [ ] Phase 7 — Monitoring, feedback and model lifecycle management.
- [ ] Phase 8 — Cloud deployment, benchmarking and documentation.

## Project Status

**Planning and initial repository setup.**

The architecture and functionality described above represent the intended implementation. Training results, benchmarks and deployment instructions will be added as functionality is implemented and verified.

## Author

**Keeno Smith**

Full-Stack Software Engineer | AI Engineer

Part of an independent software engineering portfolio focused on machine learning engineering, production-oriented AI systems and cloud-native application development.
