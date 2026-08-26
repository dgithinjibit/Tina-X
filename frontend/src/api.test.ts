// Tests for the API helpers: URL derivation and BOM fetch (with a mocked fetch).
import { describe, it, expect, vi } from "vitest";
import { wsUrl, fetchBom, fetchSwarm, fetchQuorum, fetchFlood, fetchGnss, REFLEX_BUDGET_US } from "./api";

describe("wsUrl", () => {
  it("converts http -> ws", () => {
    expect(wsUrl("http://127.0.0.1:8080")).toBe("ws://127.0.0.1:8080/ws");
  });
  it("converts https -> wss", () => {
    expect(wsUrl("https://tina.example.com")).toBe("wss://tina.example.com/ws");
  });
});

describe("REFLEX_BUDGET_US", () => {
  it("matches the Rust fly-derived budget (13 ms)", () => {
    expect(REFLEX_BUDGET_US).toBe(13_000);
  });
});

describe("fetchBom", () => {
  it("returns parsed JSON on success", async () => {
    const payload = { items: [], total_est_cost_usd: 0 };
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, json: async () => payload })) as unknown as typeof fetch,
    );
    await expect(fetchBom("http://x")).resolves.toEqual(payload);
    vi.unstubAllGlobals();
  });

  it("throws on a non-ok response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: false, status: 500 })) as unknown as typeof fetch,
    );
    await expect(fetchBom("http://x")).rejects.toThrow("500");
    vi.unstubAllGlobals();
  });
});

describe("fetchSwarm", () => {
  it("returns the parsed swarm snapshot on success", async () => {
    const payload = {
      width: 2,
      height: 1,
      tick: 3,
      leader: 1,
      leader_count: 1,
      coverage_fraction: 1,
      rho: 0.05,
      agents: [],
      pheromone: [0.1, 0.9],
      coverage: [3, 4],
    };
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, json: async () => payload })) as unknown as typeof fetch,
    );
    await expect(fetchSwarm("http://x")).resolves.toEqual(payload);
    vi.unstubAllGlobals();
  });

  it("throws on a non-ok response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: false, status: 503 })) as unknown as typeof fetch,
    );
    await expect(fetchSwarm("http://x")).rejects.toThrow("503");
    vi.unstubAllGlobals();
  });
});

describe("fetchQuorum", () => {
  it("returns the parsed quorum verdict on success", async () => {
    const payload = {
      available: true,
      risk: "high",
      candidates: [{ id: 63, support: 64, inhibition: 0 }],
      verdict: "commit",
      committed_id: 63,
      weight: 64,
    };
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, json: async () => payload })) as unknown as typeof fetch,
    );
    await expect(fetchQuorum("http://x")).resolves.toEqual(payload);
    vi.unstubAllGlobals();
  });

  it("throws on a non-ok response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: false, status: 500 })) as unknown as typeof fetch,
    );
    await expect(fetchQuorum("http://x")).rejects.toThrow("500");
    vi.unstubAllGlobals();
  });
});

describe("fetchFlood", () => {
  it("returns the parsed flood overlay on success", async () => {
    const payload = {
      width: 2,
      height: 1,
      source: "fixture",
      risk: [0, 1],
      mean_risk: 0.5,
      reaches: [{ reach_id: 7, x: 1, y: 0, forecast_cms: 300, threshold_cms: 100, risk: 1 }],
      coverage_fraction: 1,
    };
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, json: async () => payload })) as unknown as typeof fetch,
    );
    await expect(fetchFlood("http://x")).resolves.toEqual(payload);
    vi.unstubAllGlobals();
  });

  it("throws on a non-ok response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: false, status: 503 })) as unknown as typeof fetch,
    );
    await expect(fetchFlood("http://x")).rejects.toThrow("503");
    vi.unstubAllGlobals();
  });
});

describe("fetchGnss", () => {
  it("returns the parsed GNSS track on success", async () => {
    const payload = {
      width: 8,
      height: 8,
      source: "fixture",
      fixes: [
        { lat: 48.117, lon: 11.517, valid: true, cell: [1, 2] },
        { lat: 48.13, lon: 11.54, valid: false, cell: null },
      ],
      current_cell: [1, 2],
    };
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, json: async () => payload })) as unknown as typeof fetch,
    );
    await expect(fetchGnss("http://x")).resolves.toEqual(payload);
    vi.unstubAllGlobals();
  });

  it("throws on a non-ok response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: false, status: 503 })) as unknown as typeof fetch,
    );
    await expect(fetchGnss("http://x")).rejects.toThrow("503");
    vi.unstubAllGlobals();
  });
});
