import { Link } from "react-router-dom";
import { Plus, RefreshCw } from "lucide-react";
import {
  PageHeader,
  Stats,
  Distribution,
  InspectionTable,
  Loading,
  ErrorState,
  Empty,
  SyntheticNotice,
  useResource,
  percent,
} from "../components/common";
import type { Metrics } from "../types";
export default function Overview() {
  const r = useResource<Metrics>("/api/overview");
  return (
    <>
      <PageHeader
        eyebrow="THE WORKSPACE"
        title="A clearer view of quality."
        description="From inspection to insight. Classify known surface patterns, review the evidence and trace every model decision."
        action={
          <Link className="button" to="/inspections">
            <Plus size={16} /> New inspection
          </Link>
        }
      />
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
                { label: "Total inspections", value: r.data.total },
                { label: "Awaiting review", value: r.data.awaiting_review },
                {
                  label: "Mean inference",
                  value: `${r.data.mean_latency_ms.toFixed(1)} ms`,
                },
                {
                  label: "Reviewed agreement",
                  value:
                    r.data.reviewed_agreement === null
                      ? "—"
                      : percent(r.data.reviewed_agreement),
                  detail: `${r.data.reviewed} reviewed labels · selected sample`,
                },
              ]}
            />
            <div className="two-columns">
              <section className="panel">
                <div className="panel-title">
                  <h2>Pattern distribution</h2>
                  <span className="muted">Stored predictions</span>
                </div>
                <Distribution counts={r.data.class_counts} />
              </section>
              <section className="panel model-summary">
                <p className="eyebrow">INFERENCE ENGINE</p>
                <h2>{r.data.active?.version || "Awaiting a model"}</h2>
                <span className={`badge ${r.data.model_loaded ? "good" : ""}`}>
                  {r.data.model_loaded ? "Session loaded" : "Unavailable"}
                </span>
                <p>
                  Evaluation is measured on held-out data. Prediction agreement
                  uses human-reviewed labels only.
                </p>
                {r.data.active_evaluation && (
                  <div className="mini-stats">
                    <span>
                      Test macro-F1{" "}
                      <b>{percent(r.data.active_evaluation.macro_f1)}</b>
                    </span>
                    <span>
                      Low confidence <b>{r.data.low_confidence}</b>
                    </span>
                  </div>
                )}
                <Link className="text-link" to="/models">
                  Explore Model Lab →
                </Link>
              </section>
            </div>
            <section className="panel">
              <div className="panel-title">
                <h2>Recent inspections</h2>
                <button
                  className="icon-button"
                  aria-label="Refresh overview"
                  onClick={r.reload}
                >
                  <RefreshCw size={17} />
                </button>
              </div>
              {r.data.recent.length ? (
                <InspectionTable rows={r.data.recent} />
              ) : (
                <Empty>
                  Your inspection history starts with an image.
                  <br />
                  <Link to="/inspections">Upload your first inspection →</Link>
                </Empty>
              )}
            </section>
          </>
        )
      )}
    </>
  );
}
