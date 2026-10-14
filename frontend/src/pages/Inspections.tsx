import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Upload, ChevronLeft, ChevronRight } from "lucide-react";
import { request } from "../services/api";
import { classes, human, type Page, type Inspection } from "../types";
import {
  PageHeader,
  InspectionTable,
  Loading,
  Empty,
  ErrorState,
  useResource,
} from "../components/common";
export default function Inspections({
  reviewQueue = false,
}: {
  reviewQueue?: boolean;
}) {
  const [search, setSearch] = useState(""),
    [category, setCategory] = useState(""),
    [status, setStatus] = useState(reviewQueue ? "unreviewed" : ""),
    [low, setLow] = useState(false),
    [page, setPage] = useState(1),
    [sort, setSort] = useState("newest");
  const [file, setFile] = useState<File>(),
    [preview, setPreview] = useState(""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const navigate = useNavigate();
  const query = new URLSearchParams({
    search,
    category,
    review_status: status,
    page: String(page),
    limit: "10",
    sort,
  });
  if (!category) query.delete("category");
  if (!status) query.delete("review_status");
  if (low) query.set("max_confidence", "0.7");
  const r = useResource<Page>(`/api/inspections?${query}`);
  useEffect(() => {
    setStatus(reviewQueue ? "unreviewed" : "");
  }, [reviewQueue]);
  useEffect(() => {
    setPage(1);
  }, [search, category, status, low, sort]);
  useEffect(() => {
    if (!file) {
      setPreview("");
      return;
    }
    const url = URL.createObjectURL(file);
    setPreview(url);
    return () => URL.revokeObjectURL(url);
  }, [file]);
  function choose(f?: File) {
    setError("");
    if (
      f &&
      (!["image/png", "image/jpeg", "image/bmp"].includes(f.type) ||
        f.size > 10 * 1024 * 1024)
    ) {
      setError("Choose a PNG, JPEG or BMP under 10 MiB.");
      setFile(undefined);
      return;
    }
    setFile(f);
  }
  async function submit() {
    if (!file) {
      setError("Choose an image first.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      const form = new FormData();
      form.append("image", file);
      const item = await request<Inspection>("/api/inspections", {
        method: "POST",
        body: form,
      });
      navigate(`/inspections/${item.id}`);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <PageHeader
        eyebrow={reviewQueue ? "HUMAN OVERSIGHT" : "VISUAL INSPECTION"}
        title={
          reviewQueue
            ? "Every judgment deserves a second look."
            : "Start with the surface."
        }
        description={
          reviewQueue
            ? "Prioritize uncertain predictions, accept or correct a label and preserve the original model output."
            : "Upload an inspection image for known-defect classification. This classifier does not make industrial pass/fail decisions."
        }
      />
      {!reviewQueue && (
        <section className="panel upload-panel">
          <label
            className="dropzone"
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => {
              e.preventDefault();
              choose(e.dataTransfer.files[0]);
            }}
          >
            <input
              aria-label="Inspection image"
              type="file"
              accept="image/png,image/jpeg,image/bmp"
              onChange={(e) => choose(e.target.files?.[0])}
            />
            {preview ? (
              <img src={preview} alt="Upload preview" />
            ) : (
              <Upload size={30} strokeWidth={1.2} />
            )}
            <strong>{file?.name || "Drop an image here, or browse"}</strong>
            <span>PNG, JPEG or BMP · Up to 10 MiB · 16 megapixels maximum</span>
          </label>
          <div className="upload-actions">
            <div>
              <h2>Classify a known pattern</h2>
              <p>
                Six categories. Actual model probabilities.
                <br />
                Review the result before making a production decision.
              </p>
            </div>
            <button disabled={busy || !file} onClick={submit}>
              {busy ? "Running inference…" : "Analyze image"}
            </button>
          </div>
          {error && <ErrorState message={error} />}
        </section>
      )}
      <section className="panel">
        <div className="panel-title">
          <h2>{reviewQueue ? "Review queue" : "Inspection history"}</h2>
          <span className="muted">{r.data?.total ?? "—"} records</span>
        </div>
        <div className="filters">
          <input
            aria-label="Search inspections"
            placeholder="Search ID or pattern"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <select
            aria-label="Category filter"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          >
            <option value="">All patterns</option>
            {classes.map((c) => (
              <option key={c} value={c}>
                {human(c)}
              </option>
            ))}
          </select>
          <select
            aria-label="Review status filter"
            value={status}
            onChange={(e) => setStatus(e.target.value)}
          >
            <option value="">All reviews</option>
            <option value="unreviewed">Awaiting review</option>
            <option value="reviewed">Reviewed</option>
          </select>
          <select
            aria-label="Date sorting"
            value={sort}
            onChange={(e) => setSort(e.target.value)}
          >
            <option value="newest">Newest first</option>
            <option value="oldest">Oldest first</option>
          </select>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={low}
              onChange={(e) => setLow(e.target.checked)}
            />{" "}
            Low confidence
          </label>
        </div>
        {r.loading ? (
          <Loading />
        ) : r.error ? (
          <ErrorState message={r.error} retry={r.reload} />
        ) : r.data?.items.length ? (
          <InspectionTable rows={r.data.items} />
        ) : (
          <Empty>
            {reviewQueue
              ? "No inspections match this review queue."
              : "No inspections match these filters. Upload an image to begin."}
          </Empty>
        )}
        <div className="pagination">
          <button
            className="secondary"
            aria-label="Previous page"
            disabled={page === 1}
            onClick={() => setPage(page - 1)}
          >
            <ChevronLeft size={16} />
          </button>
          <span>Page {page}</span>
          <button
            className="secondary"
            aria-label="Next page"
            disabled={!r.data || page * 10 >= r.data.total}
            onClick={() => setPage(page + 1)}
          >
            <ChevronRight size={16} />
          </button>
        </div>
      </section>
    </>
  );
}
