import { beforeEach, describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Routes, Route } from "react-router-dom";
import Inspections from "./Inspections";
import Detail from "./Detail";
import Models from "./Models";
import { classes, type Inspection, type Model } from "../types";
import { setOperatorToken } from "../services/api";
const item: Inspection = {
  id: "test-record",
  image_key: "images/test.png",
  created_at: "2026-10-14T10:00:00Z",
  review_status: "unreviewed",
  review: null,
  status: "completed",
  total_ms: 3,
  prediction: {
    label: "crazing",
    confidence: 0.82,
    probabilities: [0.82, 0.04, 0.04, 0.03, 0.03, 0.04],
    model_version: "smoke-a",
    dataset_version: "a".repeat(64),
    synthetic: true,
    inference_ms: 1.5,
  },
};
const metric = {
  accuracy: 0.8,
  macro_f1: 0.75,
  weighted_f1: 0.8,
  validation_loss: 0.3,
  onnx_latency_ms: 1,
  onnx_p95_ms: 2,
  test_count: 36,
  per_class: Object.fromEntries(
    classes.map((c) => [
      c,
      { precision: 0.8, recall: 0.8, "f1-score": 0.8, support: 6 },
    ]),
  ),
  confusion_matrix: classes.map((_, i) =>
    classes.map((_, j) => (i === j ? 6 : 0)),
  ),
};
const model: Model = {
  status: "candidate",
  gates: null,
  metadata: {
    version: "smoke-a",
    dataset_version: "a".repeat(64),
    test_split_hash: "b".repeat(64),
    framework: "TensorFlow",
    architecture: "surface_cnn",
    started_at: item.created_at,
    finished_at: item.created_at,
    training_config: { epochs: 2 },
    artifact_bytes: 1000,
    provenance: { source: "procedural", dataset_id: "smoke", synthetic: true },
    history: {
      loss: [1, 0.4],
      val_loss: [1, 0.5],
      accuracy: [0.3, 0.8],
      val_accuracy: [0.2, 0.7],
    },
    evaluation: metric,
    parity: { passed: true, max_absolute_error: 1e-7 },
  },
};
function mockResponses(handler: (url: string, init?: RequestInit) => unknown) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const body = handler(url, init);
      if (body instanceof Error)
        return {
          ok: false,
          status: 503,
          json: async () => ({ error: { message: body.message } }),
        };
      return { ok: true, status: 200, json: async () => body };
    }),
  );
}
function mount(node: React.ReactNode, path = "/") {
  return render(<MemoryRouter initialEntries={[path]}>{node}</MemoryRouter>);
}
beforeEach(() => {
  vi.restoreAllMocks();
  setOperatorToken("test-operator-token");
  vi.stubGlobal("URL", {
    ...URL,
    createObjectURL: () => "blob:preview",
    revokeObjectURL: vi.fn(),
  });
});
describe("inspection workflows", () => {
  it("uploads image and renders actual API prediction", async () => {
    mockResponses((url, init) =>
      url.includes("/reviews")
        ? []
        : init?.method === "POST"
          ? item
          : url.includes("test-record")
            ? item
            : { items: [], total: 0, page: 1, limit: 10 },
    );
    mount(
      <Routes>
        <Route path="/" element={<Inspections />} />
        <Route path="/inspections/:id" element={<Detail />} />
      </Routes>,
    );
    await userEvent.upload(
      screen.getByLabelText("Inspection image"),
      new File(["pixels"], "surface.png", { type: "image/png" }),
    );
    await userEvent.click(
      screen.getByRole("button", { name: "Analyze image" }),
    );
    expect(
      await screen.findByText("Original model prediction", { exact: false }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("82.0%", { selector: "strong" }),
    ).toBeInTheDocument();
    const call = vi
      .mocked(fetch)
      .mock.calls.find((c) => c[1]?.method === "POST");
    expect(call?.[1]?.body).toBeInstanceOf(FormData);
  });
  it("shows validation errors before sending oversized images", async () => {
    mockResponses(() => ({ items: [], total: 0, page: 1, limit: 10 }));
    mount(<Inspections />);
    const file = new File(["a"], "large.png", { type: "image/png" });
    Object.defineProperty(file, "size", { value: 11 * 1024 * 1024 });
    await userEvent.upload(screen.getByLabelText("Inspection image"), file);
    expect(await screen.findByRole("alert")).toHaveTextContent("under 10 MiB");
    expect(
      screen.getByRole("button", { name: "Analyze image" }),
    ).toBeDisabled();
  });
  it("renders persisted history and filtered requests", async () => {
    mockResponses(() => ({ items: [item], total: 1, page: 1, limit: 10 }));
    mount(<Inspections reviewQueue />);
    expect(await screen.findByText("test-rec")).toBeInTheDocument();
    await userEvent.selectOptions(
      screen.getByLabelText("Category filter"),
      "inclusion",
    );
    await waitFor(() =>
      expect(
        vi
          .mocked(fetch)
          .mock.calls.some((c) => String(c[0]).includes("category=inclusion")),
      ).toBe(true),
    );
  });
  it("preserves prediction and sends separate review", async () => {
    mockResponses((url, init) =>
      url.includes("/reviews")
        ? init?.method === "POST"
          ? { ...item, review_status: "reviewed" }
          : []
        : item,
    );
    mount(
      <Routes>
        <Route path="/inspections/:id" element={<Detail />} />
      </Routes>,
      "/inspections/test-record",
    );
    await screen.findByLabelText("Reviewed label");
    await userEvent.selectOptions(
      screen.getByLabelText("Reviewed label"),
      "scratches",
    );
    await userEvent.type(
      screen.getByLabelText("Review note"),
      "Verified streaks",
    );
    await userEvent.click(screen.getByRole("button", { name: "Save review" }));
    expect(
      await screen.findByText("Review saved. Original prediction preserved."),
    ).toBeInTheDocument();
    const call = vi
      .mocked(fetch)
      .mock.calls.find((c) => c[1]?.method === "POST");
    expect(JSON.parse(String(call?.[1]?.body))).toMatchObject({
      label: "scratches",
      note: "Verified streaks",
    });
    expect(new Headers(call?.[1]?.headers).get("Authorization")).toContain(
      "test-operator-token",
    );
  });
  it("renders backend failure and retry", async () => {
    mockResponses(() => new Error("Firestore unavailable"));
    mount(<Inspections />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Firestore unavailable",
    );
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(2));
  });
});
describe("Model Lab", () => {
  it("selects a version and exposes real evaluation plus release gates", async () => {
    mockResponses((_url, init) =>
      init?.method === "POST"
        ? {
            ...model,
            status: "rejected",
            gates: {
              passed: false,
              reasons: ["Recall below threshold"],
              checked_at: item.created_at,
            },
          }
        : {
            items: [
              model,
              { ...model, metadata: { ...model.metadata, version: "smoke-b" } },
            ],
            active: null,
          },
    );
    mount(<Models />);
    expect(await screen.findByText("75.0%")).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", { name: /smoke-b candidate/ }),
    );
    await userEvent.click(
      screen.getByRole("button", { name: "Run release gates" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Recall below threshold",
    );
    expect(
      vi
        .mocked(fetch)
        .mock.calls.some((c) => String(c[0]).includes("/smoke-b/validate")),
    ).toBe(true);
  });
});
