import { useState } from "react";
import { post } from "../services/api";
import type { Metrics, Release, Snapshot } from "../types";
import {
  PageHeader,
  Stats,
  Distribution,
  Loading,
  Empty,
  ErrorState,
  SyntheticNotice,
  useResource,
  percent,
  date,
} from "../components/common";
export default function Monitoring() {
  const r = useResource<Metrics>("/api/monitoring"),
    releases = useResource<Release[]>("/api/releases"),
    snapshots = useResource<Snapshot[]>("/api/monitoring/snapshots");
  const [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  async function capture() {
    setBusy(true);
    setError("");
    try {
      await post("/api/monitoring/snapshots");
      snapshots.reload();
      r.reload();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <PageHeader
        eyebrow="MLOPS & MONITORING"
        title="Keep the model accountable."
        description="Observed predictions, runtime measurements, review feedback and a traceable release history."
        action={
          <button disabled={busy || !r.data} onClick={capture}>
            {busy ? "Capturing…" : "Capture snapshot"}
          </button>
        }
      />
      {error && <ErrorState message={error} />}{" "}
      {r.loading ? (
        <Loading />
      ) : r.error ? (
        <ErrorState message={r.error} retry={r.reload} />
      ) : (
        r.data && (
          <>
            {r.data.synthetic && <SyntheticNotice />}
            <Stats
              items={[
                {
                  label: "Active model",
                  value: r.data.active?.version || "—",
                  detail: `Previous: ${r.data.active?.previous || "—"} · ${r.data.model_loaded ? "Session loaded" : "Unavailable"}`,
                },
                {
                  label: "P95 inference",
                  value: `${r.data.p95_latency_ms.toFixed(2)} ms`,
                },
                {
                  label: "Process error rate",
                  value: percent(r.data.process_telemetry.error_rate),
                  detail: `${r.data.process_telemetry.failures}/${r.data.process_telemetry.requests} since startup`,
                },
                {
                  label: "Review corrections",
                  value: r.data.reviewed
                    ? percent(r.data.corrections / r.data.reviewed)
                    : "—",
                  detail: `${r.data.corrections} of ${r.data.reviewed} reviewed inspections`,
                },
              ]}
            />
            <div className="two-columns">
              <section className="panel">
                <h2>Prediction distribution</h2>
                <Distribution counts={r.data.class_counts} />
              </section>
              <section className="panel">
                <h2>Confidence distribution</h2>
                <div className="histogram">
                  {r.data.confidence_bins.map((n, i) => (
                    <div key={i} title={`${i * 10}–${(i + 1) * 10}%: ${n}`}>
                      <span>{n}</span>
                      <div
                        style={{
                          height: `${Math.max(2, (n / Math.max(1, ...r.data!.confidence_bins)) * 110)}px`,
                        }}
                      />
                      <small>{i * 10}</small>
                    </div>
                  ))}
                </div>
                <p className="muted">
                  Confidence is the model's probability, not observed accuracy.
                </p>
              </section>
            </div>
            <section className="panel">
              <h2>Distribution shift</h2>
              <p>
                {r.data.active_model_metrics.psi === null
                  ? "Collect at least 30 inspections with the active model before calculating class-frequency PSI."
                  : `Class-frequency PSI: ${r.data.active_model_metrics.psi.toFixed(3)} · ${r.data.active_model_metrics.drift_warning ? "Investigate distribution shift" : "No threshold warning"}`}
              </p>
              <p className="muted">
                Compared with the active model's held-out predicted
                distribution. PSI above 0.2 flags investigation; it does not
                prove model degradation.
              </p>
            </section>
            <section className="panel">
              <h2>Recent inference latency</h2>
              {r.data.latency_trend.length ? (
                <div className="latency-chart">
                  {r.data.latency_trend.map((point, i) => (
                    <div
                      key={i}
                      title={`${date(point.at)}: ${point.ms.toFixed(2)} ms`}
                      style={{
                        height: `${Math.max(3, (point.ms / Math.max(1, ...r.data!.latency_trend.map((p) => p.ms))) * 110)}px`,
                      }}
                    />
                  ))}
                </div>
              ) : (
                <Empty>No inference measurements yet.</Empty>
              )}
              <small>
                Up to 50 stored inspections · Oldest to newest · Hover for
                measured latency
              </small>
            </section>
          </>
        )
      )}
      <section className="panel">
        <h2>Release history</h2>
        {releases.error ? (
          <ErrorState message={releases.error} retry={releases.reload} />
        ) : releases.loading ? (
          <Loading />
        ) : releases.data?.length ? (
          <div className="timeline">
            {releases.data.map((release, i) => (
              <article key={i}>
                <span className="badge">{release.action}</span>
                <strong>{release.version}</strong>
                <small>{date(release.at)}</small>
                {release.gates && (
                  <p>
                    {release.gates.passed
                      ? "Gate validation passed"
                      : release.gates.reasons.join(" · ")}
                  </p>
                )}
              </article>
            ))}
          </div>
        ) : (
          <Empty>No release events recorded.</Empty>
        )}
      </section>
      <section className="panel">
        <h2>Monitoring snapshots</h2>
        {snapshots.error ? (
          <ErrorState message={snapshots.error} retry={snapshots.reload} />
        ) : snapshots.data?.length ? (
          <div className="timeline">
            {snapshots.data.map((s) => (
              <details key={s.id}>
                <summary>
                  {date(s.created_at)} · {s.metrics.total} inspections ·{" "}
                  {s.metrics.active?.version || "No model"}
                </summary>
                <pre>{JSON.stringify(s.metrics, null, 2)}</pre>
              </details>
            ))}
          </div>
        ) : (
          <Empty>Capture a snapshot to preserve current measurements.</Empty>
        )}
      </section>
    </>
  );
}
