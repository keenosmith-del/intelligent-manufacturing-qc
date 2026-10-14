import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { RefreshCw } from "lucide-react";
import { request } from "../services/api";
import { classes, human, type Inspection } from "../types";
import { imageUrl } from "../services/api";

export function useResource<T>(path: string) {
  const [data, setData] = useState<T>();
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [revision, setRevision] = useState(0);
  const reload = useCallback(() => setRevision((n) => n + 1), []);
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError("");
    request<T>(path)
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e.message));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [path, revision]);
  return { data, error, loading, reload };
}
export function PageHeader({
  eyebrow,
  title,
  description,
  action,
}: {
  eyebrow: string;
  title: string;
  description: string;
  action?: React.ReactNode;
}) {
  return (
    <header className="page-header">
      <div>
        <p className="eyebrow">{eyebrow}</p>
        <h1>{title}</h1>
        <p className="lede">{description}</p>
      </div>
      {action}
    </header>
  );
}
export function ErrorState({
  message,
  retry,
}: {
  message: string;
  retry?: () => void;
}) {
  return (
    <div role="alert" className="notice error">
      <p>{message}</p>
      {retry && (
        <button className="secondary" onClick={retry}>
          <RefreshCw size={15} /> Try again
        </button>
      )}
    </div>
  );
}
export function Loading() {
  return (
    <div className="loading" role="status">
      <span className="pulse" /> Loading records…
    </div>
  );
}
export function Empty({ children }: { children: React.ReactNode }) {
  return <div className="empty">{children}</div>;
}
export function SyntheticNotice() {
  return (
    <div className="notice">
      Synthetic smoke model · Procedural images verify the pipeline. Evaluation
      does not represent industrial performance.
    </div>
  );
}
export const percent = (n: number) => `${(n * 100).toFixed(1)}%`;
export const date = (s: string) => new Date(s).toLocaleString();
export function Stats({
  items,
}: {
  items: { label: string; value: React.ReactNode; detail?: string }[];
}) {
  return (
    <div className="stats">
      {items.map((i) => (
        <div key={i.label}>
          <p>{i.label}</p>
          <strong>{i.value}</strong>
          {i.detail && <small>{i.detail}</small>}
        </div>
      ))}
    </div>
  );
}
export function Distribution({ counts }: { counts: Record<string, number> }) {
  const max = Math.max(1, ...Object.values(counts));
  return (
    <div className="distribution">
      {classes.map((c) => (
        <div className="bar-row" key={c}>
          <span>{human(c)}</span>
          <div className="bar-track">
            <div style={{ width: `${((counts[c] || 0) / max) * 100}%` }} />
          </div>
          <b>{counts[c] || 0}</b>
        </div>
      ))}
    </div>
  );
}
export function Probabilities({ values }: { values: number[] }) {
  return (
    <div className="distribution">
      {classes.map((c, i) => (
        <div className="bar-row" key={c}>
          <span>{human(c)}</span>
          <div className="bar-track">
            <div style={{ width: `${values[i] * 100}%` }} />
          </div>
          <b>{percent(values[i])}</b>
        </div>
      ))}
    </div>
  );
}
export function InspectionTable({ rows }: { rows: Inspection[] }) {
  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th>Inspection</th>
            <th>Prediction</th>
            <th>Confidence</th>
            <th>Model</th>
            <th>Review</th>
            <th>Received</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.id}>
              <td>
                <Link className="inspection-link" to={`/inspections/${r.id}`}>
                  <img src={imageUrl(r.id)} alt="Inspection surface" />
                  {r.id.slice(0, 8)}
                </Link>
              </td>
              <td>{human(r.prediction.label)}</td>
              <td>{percent(r.prediction.confidence)}</td>
              <td>{r.prediction.model_version}</td>
              <td>
                <span className={`badge ${r.review ? "good" : ""}`}>
                  {human(r.review_status)}
                </span>
              </td>
              <td>{date(r.created_at)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
