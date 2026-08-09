// Component tests for the GNSS panel. It fetches /api/gnss on mount, so we mock fetch and wait for
// the grid to appear. We assert the honest error state too (server down).
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { GnssPanel } from "./GnssPanel";
import type { GnssView } from "../api";

const track: GnssView = {
  width: 4,
  height: 4,
  source: "fixture",
  fixes: [
    { lat: 48.117, lon: 11.517, valid: true, cell: [0, 0] },
    { lat: 48.13, lon: 11.54, valid: true, cell: [2, 2] },
    { lat: 48.2, lon: 11.6, valid: false, cell: null },
  ],
  current_cell: [2, 2],
};

function stubFetch(ok: boolean) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      ok ? { ok: true, json: async () => track } : { ok: false, status: 503 },
    ) as unknown as typeof fetch,
  );
}

afterEach(() => vi.unstubAllGlobals());

describe("GnssPanel", () => {
  it("renders the grid and track stats once loaded", async () => {
    stubFetch(true);
    render(<GnssPanel />);

    // Grid appears with width*height cells.
    const grid = await screen.findByTestId("gnss-grid");
    expect(grid.children).toHaveLength(track.width * track.height);

    // Provenance, valid-fix count, and current cell are surfaced.
    expect(screen.getByText("offline fixture")).toBeInTheDocument();
    expect(screen.getByText("2/3")).toBeInTheDocument(); // two valid of three fixes
    expect(screen.getByText("(2,2)")).toBeInTheDocument(); // current cell
  });

  it("shows an honest 'start the server' error when the GNSS fetch fails", async () => {
    stubFetch(false);
    render(<GnssPanel />);
    await waitFor(() => expect(screen.getByTestId("gnss-error")).toBeInTheDocument());
  });
});
