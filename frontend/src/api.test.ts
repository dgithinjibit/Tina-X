// Tests for the API helpers: URL derivation and BOM fetch (with a mocked fetch).
import { describe, it, expect, vi } from "vitest";
import { wsUrl, fetchBom, REFLEX_BUDGET_US } from "./api";

describe("wsUrl", () => {
  it("converts http -> ws", () => {
    expect(wsUrl("http://127.0.0.1:8080")).toBe("ws://127.0.0.1:8080/ws");
  });
  it("converts https -> wss", () => {
    expect(wsUrl("https://nzi.example.com")).toBe("wss://nzi.example.com/ws");
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
