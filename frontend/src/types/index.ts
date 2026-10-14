export const classes = [
  "crazing",
  "inclusion",
  "patches",
  "pitted_surface",
  "rolled_in_scale",
  "scratches",
] as const;
export const human = (s: string) =>
  s.replaceAll("_", " ").replace(/\b\w/g, (c) => c.toUpperCase());
export interface Review {
  id: string;
  inspection_id: string;
  label: string;
  note: string;
  reviewer: string;
  created_at: string;
}
export interface Inspection {
  id: string;
  image_key: string;
  created_at: string;
  review_status: string;
  review: Review | null;
  status: string;
  total_ms: number;
  prediction: {
    label: string;
    confidence: number;
    probabilities: number[];
    model_version: string;
    dataset_version: string;
    synthetic: boolean;
    inference_ms: number;
  };
}
export interface Evaluation {
  accuracy: number;
  macro_f1: number;
  weighted_f1: number;
  validation_loss: number;
  onnx_latency_ms: number;
  onnx_p95_ms: number;
  per_class: Record<
    string,
    { precision: number; recall: number; "f1-score": number; support: number }
  >;
  confusion_matrix: number[][];
  test_count: number;
}
export interface Model {
  status: string;
  gates: null | { passed: boolean; reasons: string[]; checked_at: string };
  metadata: {
    version: string;
    dataset_version: string;
    test_split_hash: string;
    framework: string;
    architecture: string;
    started_at: string;
    finished_at: string;
    training_config: Record<string, number>;
    artifact_bytes: number;
    provenance: { source: string; dataset_id: string; synthetic: boolean };
    history: Record<string, number[]>;
    evaluation: Evaluation;
    parity: { passed: boolean; max_absolute_error: number };
  };
}
export interface Active {
  version: string;
  previous: string | null;
  activated_at: string;
}
export interface ModelList {
  items: Model[];
  active: Active | null;
}
export interface Metrics {
  total: number;
  awaiting_review: number;
  reviewed: number;
  low_confidence: number;
  mean_latency_ms: number;
  p95_latency_ms: number;
  corrections: number;
  reviewed_agreement: number | null;
  class_counts: Record<string, number>;
  confidence_bins: number[];
  active: Active | null;
  active_evaluation: Evaluation | null;
  synthetic: boolean | null;
  model_loaded: boolean;
  recent: Inspection[];
  active_model_metrics: {
    psi: number | null;
    drift_warning: boolean;
    total: number;
  };
  process_telemetry: {
    requests: number;
    failures: number;
    error_rate: number;
    scope: string;
  };
  latency_trend: { at: string; ms: number }[];
}
export interface Page {
  items: Inspection[];
  total: number;
  page: number;
  limit: number;
}
export interface Release {
  action: string;
  version: string;
  at: string;
  previous?: string;
  gates?: { passed: boolean; reasons: string[] };
}
export interface Snapshot {
  id: string;
  created_at: string;
  metrics: Metrics;
}
