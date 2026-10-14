import { classes, human, type Model } from "../types";
import { percent } from "./common";

export function ClassTable({ model }: { model: Model }) {
  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th>Pattern</th>
            <th>Precision</th>
            <th>Recall</th>
            <th>F1</th>
          </tr>
        </thead>
        <tbody>
          {classes.map((c) => {
            const v = model.metadata.evaluation.per_class[c];
            return (
              <tr key={c}>
                <td>{human(c)}</td>
                <td>{percent(v.precision)}</td>
                <td>{percent(v.recall)}</td>
                <td>{percent(v["f1-score"])}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
export function Confusion({ model }: { model: Model }) {
  const matrix = model.metadata.evaluation.confusion_matrix;
  const max = Math.max(1, ...matrix.flat());
  return (
    <div className="confusion" aria-label="Confusion matrix">
      {matrix.map((row, i) =>
        row.map((n, j) => (
          <div
            title={`${human(classes[i])} → ${human(classes[j])}: ${n}`}
            key={`${i}-${j}`}
            style={{ background: `rgba(57,84,70,${0.04 + (n / max) * 0.35})` }}
          >
            {n}
          </div>
        )),
      )}
    </div>
  );
}
export function History({ values }: { values: Record<string, number[]> }) {
  return (
    <>
      <div className="two-columns">
        <Curve
          title="Accuracy"
          train={values.accuracy || []}
          validation={values.val_accuracy || []}
        />
        <Curve
          title="Loss"
          train={values.loss || []}
          validation={values.val_loss || []}
        />
      </div>
      <div className="table-scroll history">
        <table>
          <thead>
            <tr>
              <th>Epoch</th>
              <th>Train accuracy</th>
              <th>Validation accuracy</th>
              <th>Train loss</th>
              <th>Validation loss</th>
            </tr>
          </thead>
          <tbody>
            {values.loss?.map((v, i) => (
              <tr key={i}>
                <td>{i + 1}</td>
                <td>{percent(values.accuracy[i])}</td>
                <td>{percent(values.val_accuracy[i])}</td>
                <td>{v.toFixed(3)}</td>
                <td>{values.val_loss[i].toFixed(3)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
function Curve({
  title,
  train,
  validation,
}: {
  title: string;
  train: number[];
  validation: number[];
}) {
  const max = Math.max(1, ...train, ...validation);
  const points = (values: number[]) =>
    values
      .map(
        (v, i) =>
          `${12 + (i / Math.max(1, values.length - 1)) * 296},${140 - (v / max) * 120}`,
      )
      .join(" ");
  return (
    <figure className="training-curve">
      <figcaption>{title} · training / validation</figcaption>
      <svg viewBox="0 0 320 160" role="img" aria-label={`${title} by epoch`}>
        <path d="M12 14V140H308" fill="none" stroke="#e2e7da" />
        <polyline
          points={points(train)}
          fill="none"
          stroke="#59764b"
          strokeWidth="2"
        />
        <polyline
          points={points(validation)}
          fill="none"
          stroke="#baa780"
          strokeWidth="2"
        />
      </svg>
      <small>Epoch 1 → {train.length} · Green: train · Sand: validation</small>
    </figure>
  );
}
export function Comparison({ a, b }: { a: Model; b: Model }) {
  const compatible =
    a.metadata.dataset_version === b.metadata.dataset_version &&
    a.metadata.test_split_hash === b.metadata.test_split_hash;
  return (
    <>
      {!compatible && (
        <div className="notice error">
          Dataset or held-out split differs. Quality comparisons are withheld.
        </div>
      )}
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Measurement</th>
              <th>{a.metadata.version}</th>
              <th>{b.metadata.version}</th>
              <th>Delta (second − first)</th>
            </tr>
          </thead>
          <tbody>
            {compatible && (
              <>
                {(["accuracy", "macro_f1"] as const).map((k) => (
                  <tr key={k}>
                    <td>{human(k)}</td>
                    <td>{percent(a.metadata.evaluation[k])}</td>
                    <td>{percent(b.metadata.evaluation[k])}</td>
                    <td>
                      {(
                        (b.metadata.evaluation[k] - a.metadata.evaluation[k]) *
                        100
                      ).toFixed(2)}{" "}
                      pp
                    </td>
                  </tr>
                ))}
                {classes.map((c) => (
                  <tr key={c}>
                    <td>{human(c)} recall</td>
                    <td>
                      {percent(a.metadata.evaluation.per_class[c].recall)}
                    </td>
                    <td>
                      {percent(b.metadata.evaluation.per_class[c].recall)}
                    </td>
                    <td>
                      {(
                        (b.metadata.evaluation.per_class[c].recall -
                          a.metadata.evaluation.per_class[c].recall) *
                        100
                      ).toFixed(2)}{" "}
                      pp
                    </td>
                  </tr>
                ))}
              </>
            )}
            <tr>
              <td>ONNX latency</td>
              <td>{a.metadata.evaluation.onnx_latency_ms.toFixed(2)} ms</td>
              <td>{b.metadata.evaluation.onnx_latency_ms.toFixed(2)} ms</td>
              <td>Hardware-dependent measurement</td>
            </tr>
            <tr>
              <td>Artifact bytes</td>
              <td>{a.metadata.artifact_bytes}</td>
              <td>{b.metadata.artifact_bytes}</td>
              <td>{b.metadata.artifact_bytes - a.metadata.artifact_bytes}</td>
            </tr>
            <tr>
              <td>Release status</td>
              <td>{a.status}</td>
              <td>{b.status}</td>
              <td>—</td>
            </tr>
          </tbody>
        </table>
      </div>
    </>
  );
}
