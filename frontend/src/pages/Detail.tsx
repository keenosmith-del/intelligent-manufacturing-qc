import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { post, imageUrl } from "../services/api";
import { classes, human, type Inspection, type Review } from "../types";
import {
  PageHeader,
  Probabilities,
  Stats,
  Loading,
  ErrorState,
  SyntheticNotice,
  useResource,
  percent,
  date,
} from "../components/common";
export default function Detail() {
  const { id } = useParams();
  const r = useResource<Inspection>(`/api/inspections/${id}`);
  const history = useResource<Review[]>(`/api/inspections/${id}/reviews`);
  const [label, setLabel] = useState(""),
    [note, setNote] = useState(""),
    [reviewer, setReviewer] = useState("Local operator"),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [saved, setSaved] = useState(false);
  async function save() {
    if (!r.data) return;
    setBusy(true);
    setSaved(false);
    setError("");
    try {
      await post(`/api/inspections/${id}/reviews`, {
        label: label || r.data.prediction.label,
        note,
        reviewer,
      });
      r.reload();
      history.reload();
      setSaved(true);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Link className="text-link" to="/inspections">
        ← Inspection history
      </Link>
      <PageHeader
        eyebrow="INSPECTION RECORD"
        title={r.data ? human(r.data.prediction.label) : "Inspection detail"}
        description={`Record ${id}`}
      />
      {r.loading ? (
        <Loading />
      ) : r.error ? (
        <ErrorState message={r.error} retry={r.reload} />
      ) : (
        r.data && (
          <>
            {r.data.prediction.synthetic && <SyntheticNotice />}
            <div className="two-columns detail-grid">
              <section className="panel evidence">
                <img
                  src={imageUrl(r.data.id)}
                  alt="Uploaded inspection evidence"
                />
                <p>Original image · {date(r.data.created_at)}</p>
              </section>
              <section className="panel">
                <p className="eyebrow">ORIGINAL MODEL PREDICTION</p>
                <h2>{human(r.data.prediction.label)}</h2>
                <Stats
                  items={[
                    {
                      label: "Confidence",
                      value: percent(r.data.prediction.confidence),
                    },
                    {
                      label: "Inference",
                      value: `${r.data.prediction.inference_ms.toFixed(2)} ms`,
                    },
                  ]}
                />
                <Probabilities values={r.data.prediction.probabilities} />
                <p className="metadata">
                  Model: {r.data.prediction.model_version}
                  <br />
                  Dataset: {r.data.prediction.dataset_version.slice(0, 16)}
                  <br />
                  Review: {human(r.data.review_status)}
                </p>
              </section>
            </div>
            <section className="panel">
              <div className="panel-title">
                <h2>Human review</h2>
                {r.data.review && (
                  <span className="badge good">
                    Reviewed: {human(r.data.review.label)}
                  </span>
                )}
              </div>
              <p className="muted">
                Accept the original label or select a correction. Each saved
                action remains in review history.
              </p>
              <div className="review-form">
                <label>
                  Reviewed label
                  <select
                    aria-label="Reviewed label"
                    value={label || r.data.prediction.label}
                    onChange={(e) => setLabel(e.target.value)}
                  >
                    {classes.map((c) => (
                      <option key={c} value={c}>
                        {human(c)}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Reviewer
                  <input
                    aria-label="Reviewer"
                    value={reviewer}
                    maxLength={100}
                    onChange={(e) => setReviewer(e.target.value)}
                  />
                </label>
                <label className="wide">
                  Review note
                  <textarea
                    aria-label="Review note"
                    value={note}
                    maxLength={2000}
                    onChange={(e) => setNote(e.target.value)}
                    placeholder="Record the evidence behind your judgment…"
                  />
                </label>
              </div>
              <button disabled={busy || !reviewer.trim()} onClick={save}>
                {busy ? "Saving…" : "Save review"}
              </button>
              {saved && (
                <p role="status" className="success">
                  Review saved. Original prediction preserved.
                </p>
              )}
              {error && <ErrorState message={error} />}
              <div className="review-history">
                {history.error && (
                  <ErrorState message={history.error} retry={history.reload} />
                )}{" "}
                {history.data?.map((h) => (
                  <article key={h.id}>
                    <span className="badge">{human(h.label)}</span>
                    <p>{h.note || "No note added."}</p>
                    <small>
                      {h.reviewer} · {date(h.created_at)}
                    </small>
                  </article>
                ))}
              </div>
            </section>
          </>
        )
      )}
    </>
  );
}
