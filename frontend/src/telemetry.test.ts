// Unit tests for the pure telemetry helpers (no DOM needed).
import { describe, it, expect } from "vitest";
import { pushWindow, withinBudget, trackingError, computeStats } from "./telemetry";
import type { ReflexSample } from "./api";

function sample(over: Partial<ReflexSample> = {}): ReflexSample {
  return { step: 1, setpoint: 1, measured: 0.9, command: 0.1, latency_us: 10, ...over };
}

describe("pushWindow", () => {
  it("appends samples", () => {
    const out = pushWindow([], sample({ step: 1 }), 5);
    expect(out).toHaveLength(1);
    expect(out[0].step).toBe(1);
  });

  it("caps the window at max length, dropping the oldest", () => {
    let buf: ReflexSample[] = [];
    for (let i = 1; i <= 10; i++) buf = pushWindow(buf, sample({ step: i }), 5);
    expect(buf).toHaveLength(5);
    expect(buf[0].step).toBe(6); // oldest kept is step 6
    expect(buf[4].step).toBe(10);
  });
});

describe("withinBudget", () => {
  it("passes under the 13 ms budget and fails over it", () => {
    expect(withinBudget(sample({ latency_us: 12_999 }))).toBe(true);
    expect(withinBudget(sample({ latency_us: 13_001 }))).toBe(false);
  });
});

describe("trackingError", () => {
  it("is the absolute setpoint-measured difference", () => {
    expect(trackingError(sample({ setpoint: 1, measured: 0.7 }))).toBeCloseTo(0.3);
  });
});

describe("computeStats", () => {
  it("returns safe defaults for an empty window", () => {
    const s = computeStats([]);
    expect(s.count).toBe(0);
    expect(s.allWithinBudget).toBe(true);
  });

  it("aggregates latency and flags budget violations", () => {
    const buf = [sample({ latency_us: 5 }), sample({ latency_us: 20_000 }), sample({ latency_us: 7 })];
    const s = computeStats(buf);
    expect(s.count).toBe(3);
    expect(s.maxLatencyUs).toBe(20_000);
    expect(s.allWithinBudget).toBe(false); // 20 ms > 13 ms budget
  });
});
