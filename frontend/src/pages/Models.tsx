import { useState } from "react";
import { FlaskConical, RotateCcw } from "lucide-react";
import { post } from "../services/api";
import { classes, human, type ModelList, type Model } from "../types";
import {
  ClassTable,
  Confusion,
  History,
  Comparison,
} from "../components/model-evaluation";
import {
  PageHeader,
  Stats,
  Loading,
  ErrorState,
  SyntheticNotice,
  useResource,
  percent,
  date,
} from "../components/common";
export default function Models() {
  const r = useResource<ModelList>("/api/models");
  const [selected, setSelected] = useState(""),
    [compare, setCompare] = useState(""),
    [version, setVersion] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState("");
  const model =
    r.data?.items.find((m) => m.metadata.version === selected) ||
    r.data?.items[0];
  const other = r.data?.items.find((m) => m.metadata.version === compare);
  async function action(path: string, body?: unknown) {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      const result = await post<Model | { version: string }>(path, body);
      if ("status" in result && result.status === "rejected")
        setError(
          result.gates?.reasons.join(" · ") ||
            "Release gates rejected this candidate",
        );
      else
        setMessage("Operation saved. Registry and inference state refreshed.");
      r.reload();
    } catch (e) {
      setError((e as Error).message);
      r.reload();
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <PageHeader
        eyebrow="MODEL ENGINEERING"
        title="Evidence before release."
        description="Inspect training provenance, compare held-out evaluations and release an executable, validated model."
        action={<FlaskConical size={34} strokeWidth={1} />}
      />
      <section className="panel">
        <div className="panel-title">
          <h2>Model registry</h2>
          <span className="badge">
            Active: {r.data?.active?.version || "None"}
          </span>
        </div>
        <div className="register-form">
          <input
            aria-label="Artifact version"
            placeholder="Version in configured artifact directory"
            value={version}
            onChange={(e) => setVersion(e.target.value)}
          />
          <button
            disabled={busy || !version.trim()}
            onClick={() => action("/api/models", { version })}
          >
            Register artifact
          </button>
          <button
            className="secondary"
            disabled={busy || !r.data?.active?.previous}
            onClick={() => action("/api/models/rollback")}
          >
            <RotateCcw size={15} /> Rollback to{" "}
            {r.data?.active?.previous || "previous"}
          </button>
        </div>
        <p className="muted">
          Train with the offline Python CLI. Registration imports a checked
          package from the configured artifact directory.
        </p>
        {r.loading ? (
          <Loading />
        ) : r.error ? (
          <ErrorState message={r.error} retry={r.reload} />
        ) : r.data?.items.length ? (
          <div className="model-tabs">
            {r.data.items.map((m) => (
              <button
                className={
                  model?.metadata.version === m.metadata.version
                    ? "selected secondary"
                    : "secondary"
                }
                key={m.metadata.version}
                onClick={() => setSelected(m.metadata.version)}
              >
                {m.metadata.version}
                <span
                  className={`badge ${m.status === "validated" ? "good" : ""}`}
                >
                  {m.status}
                </span>
              </button>
            ))}
          </div>
        ) : (
          <p className="empty">
            No models registered. Train and export a version, then register its
            artifact.
          </p>
        )}
      </section>
      {error && <ErrorState message={error} />}{" "}
      {message && (
        <p role="status" className="success">
          {message}
        </p>
      )}
      {model && (
        <>
          {model.metadata.provenance.synthetic && <SyntheticNotice />}
          <section className="panel">
            <div className="panel-title">
              <div>
                <p className="eyebrow">{model.metadata.architecture}</p>
                <h2>{model.metadata.version}</h2>
              </div>
              <div className="button-row">
                <button
                  className="secondary"
                  disabled={busy}
                  onClick={() =>
                    action(`/api/models/${model.metadata.version}/validate`)
                  }
                >
                  Run release gates
                </button>
                <button
                  disabled={
                    busy || r.data?.active?.version === model.metadata.version
                  }
                  onClick={() =>
                    action(`/api/models/${model.metadata.version}/activate`)
                  }
                >
                  Validate & activate
                </button>
              </div>
            </div>
            <Stats
              items={[
                {
                  label: "Held-out accuracy",
                  value: percent(model.metadata.evaluation.accuracy),
                },
                {
                  label: "Macro-F1",
                  value: percent(model.metadata.evaluation.macro_f1),
                },
                {
                  label: "ONNX latency",
                  value: `${model.metadata.evaluation.onnx_latency_ms.toFixed(2)} ms`,
                },
                {
                  label: "Artifact",
                  value: `${(model.metadata.artifact_bytes / 1024).toFixed(0)} KB`,
                },
              ]}
            />
            <p className="metadata">
              Dataset: {model.metadata.provenance.dataset_id} ·{" "}
              {model.metadata.dataset_version.slice(0, 16)}
              <br />
              Framework: {model.metadata.framework} · Export: ONNX opset 17
              <br />
              Trained: {date(model.metadata.finished_at)} · Parity max error:{" "}
              {model.metadata.parity.max_absolute_error.toExponential(2)}
            </p>
            {model.gates && (
              <div className={`notice ${model.gates.passed ? "" : "error"}`}>
                <strong>
                  {model.gates.passed
                    ? "Release checks passed"
                    : "Release rejected"}
                </strong>
                {model.gates.reasons.map((reason) => (
                  <p key={reason}>{reason}</p>
                ))}
              </div>
            )}
            <div className="two-columns">
              <div>
                <h3>Class performance</h3>
                <ClassTable model={model} />
              </div>
              <div>
                <h3>Confusion matrix</h3>
                <Confusion model={model} />
                <small>
                  Rows: actual · Columns: predicted ·{" "}
                  {classes.map((c, i) => `${i + 1} ${human(c)}`).join(" / ")}
                </small>
              </div>
            </div>
            <h3>Training history</h3>
            <History values={model.metadata.history} />
            <details>
              <summary>Training configuration & provenance</summary>
              <pre>
                {JSON.stringify(
                  {
                    config: model.metadata.training_config,
                    provenance: model.metadata.provenance,
                  },
                  null,
                  2,
                )}
              </pre>
            </details>
          </section>
          <section className="panel">
            <div className="panel-title">
              <h2>Compare with {model.metadata.version}</h2>
              <select
                aria-label="Comparison model"
                value={compare}
                onChange={(e) => setCompare(e.target.value)}
              >
                <option value="">Select another model</option>
                {r.data?.items
                  .filter((m) => m !== model)
                  .map((m) => (
                    <option key={m.metadata.version}>
                      {m.metadata.version}
                    </option>
                  ))}
              </select>
            </div>
            {other ? (
              <Comparison a={model} b={other} />
            ) : (
              <p className="muted">
                Choose another registered model to inspect class differences and
                resource costs.
              </p>
            )}
          </section>
        </>
      )}
    </>
  );
}
