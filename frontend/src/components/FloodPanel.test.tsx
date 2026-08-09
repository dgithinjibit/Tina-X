// Component tests for the flood panel. It fetches /api/flood on mount, so we mock fetch and wait
// for the grid to appear. We assert the honest error state too (server down).
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { FloodPanel } from "./FloodPanel";
import type { FloodView } from "../api";

const overlay: FloodView = {
  width: 2,
  height: 2,
  source: "fixture",
  risk: [0, 0.2, 0.6, 1],
  mean_risk: 0.45,
  reaches: [
    { reach_id: 1, x: 0, y: 1, forecast_cms: 120, threshold_cms: 100, risk: 0.2 },
    { reach_id: 7, x: 1, y: 1, forecast_cms: 300, threshold_cms: 100, risk: 1 },
  ],
  coverage_fraction: 1,
};

function stubFetch(ok: boolean) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      ok ? { ok: true, json: async () => overlay } : { ok: false, status: 503 },
    ) as unknown as typeof fetch,
  );
}

afterEach(() => vi.unstubAllGlobals());

describe("FloodPanel", () => {
  it("renders the flood grid and threat stats once loaded", async () => {
    stubFetch(true);
    render(<FloodPanel />);

    // Grid appears once the fetch resolves; one cell per risk entry.
    const grid = await screen.findByTestId("flood-grid");
    expect(grid.children).toHaveLength(overlay.risk.length);

    // Provenance is shown honestly, and the mean risk is surfaced as a percentage.
    expect(screen.getByText("offline fixture")).toBeInTheDocument();
    expect(screen.getByText("45%")).toBeInTheDocument();
    // Two cells are at/over 0.5 risk -> "2/4" in-flood chip.
    expect(screen.getByText("2/4")).toBeInTheDocument();
  });

  it("shows an honest 'start the server' error when the flood fetch fails", async () => {
    stubFetch(false);
    render(<FloodPanel />);
    await waitFor(() => expect(screen.getByTestId("flood-error")).toBeInTheDocument());
  });
});
