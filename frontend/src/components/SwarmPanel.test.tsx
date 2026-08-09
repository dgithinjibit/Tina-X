// Component tests for the swarm panel. It fetches /api/swarm on mount, so we mock fetch and
// wait for the grid to appear. We assert the honest error state too (server down).
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { SwarmPanel } from "./SwarmPanel";
import type { SwarmView, QuorumView } from "../api";

const snapshot: SwarmView = {
  width: 2,
  height: 2,
  tick: 5,
  leader: 3,
  leader_count: 1,
  coverage_fraction: 1,
  rho: 0.05,
  agents: [
    { id: 0, x: 0, y: 0, alive: true, is_leader: false, distance_to_leader: 1, covering: true },
    { id: 3, x: 1, y: 1, alive: true, is_leader: true, distance_to_leader: 0, covering: true },
  ],
  pheromone: [0.2, 0.5, 0.1, 0.9],
  coverage: [3, 4, 3, 5],
};

const commitVerdict: QuorumView = {
  available: true,
  risk: "high",
  candidates: [{ id: 3, support: 4, inhibition: 0 }],
  verdict: "commit",
  committed_id: 3,
  weight: 4,
};

// URL-aware fetch mock: /api/quorum returns the verdict, everything else the swarm snapshot.
function stubFetch(swarmOk: boolean, quorum: QuorumView | "fail") {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      if (url.includes("/api/quorum")) {
        if (quorum === "fail") return { ok: false, status: 503 };
        return { ok: true, json: async () => quorum };
      }
      return swarmOk ? { ok: true, json: async () => snapshot } : { ok: false, status: 503 };
    }) as unknown as typeof fetch,
  );
}

afterEach(() => vi.unstubAllGlobals());

describe("SwarmPanel", () => {
  it("renders the grid, stats, and the quorum verdict once loaded", async () => {
    stubFetch(true, commitVerdict);
    render(<SwarmPanel />);

    // Grid appears once the fetch resolves; one cell per pheromone entry.
    const grid = await screen.findByTestId("swarm-grid");
    expect(grid.children).toHaveLength(snapshot.pheromone.length);

    // The elected brain is surfaced as a stat, and coverage is shown as a percentage.
    expect(screen.getByText("#3")).toBeInTheDocument();
    expect(screen.getByText("100%")).toBeInTheDocument();

    // The honeybee quorum verdict banner shows the commit decision.
    const banner = await screen.findByTestId("quorum-banner");
    expect(banner.textContent).toMatch(/commit to brain #3/);
  });

  it("still renders the field when the quorum arbiter is unavailable", async () => {
    // A quorum fetch failure must NOT block the field view (best-effort verdict).
    stubFetch(true, "fail");
    render(<SwarmPanel />);
    expect(await screen.findByTestId("swarm-grid")).toBeInTheDocument();
    expect(screen.queryByTestId("quorum-banner")).not.toBeInTheDocument();
  });

  it("shows an honest 'start the server' error when the swarm fetch fails", async () => {
    stubFetch(false, "fail");
    render(<SwarmPanel />);
    await waitFor(() => expect(screen.getByTestId("swarm-error")).toBeInTheDocument());
  });
});
