// Unit tests for the pure telemetry helpers (no DOM needed).
import { describe, it, expect } from "vitest";
import {
  pushWindow,
  withinBudget,
  trackingError,
  computeStats,
  hasAttitude,
  axisValues,
  axisSeries,
  axesPresent,
} from "./telemetry";
import type { AttitudeSample, ReflexSample } from "./api";

function sample(over: Partial<ReflexSample> = {}): ReflexSample {
  return { step: 1, setpoint: 1, measured: 0.9, command: 0.1, latency_us: 10, ...over };
}

function attitude(): AttitudeSample {
  return {
    setpoint: { roll: 1, pitch: -0.5, yaw: 0.25 },
    measured: { roll: 0.9, pitch: -0.3, yaw: 0.2 },
    command: { roll: 0.1, pitch: -0.05, yaw: 0.02 },
  };
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
  it("is the absolute setpoint-measured difference for a scalar sample", () => {
    expect(trackingError(sample({ setpoint: 1, measured: 0.7 }))).toBeCloseTo(0.3);
  });

  it("is the WORST-axis error when 3-axis detail is present", () => {
    // pitch error 0.2 is the largest of {roll 0.1, pitch 0.2, yaw 0.05}.
    expect(trackingError(sample({ attitude: attitude() }))).toBeCloseTo(0.2);
  });
});

describe("3-axis helpers", () => {
  it("hasAttitude reflects presence of the attitude block", () => {
    expect(hasAttitude(sample())).toBe(false);
    expect(hasAttitude(sample({ attitude: attitude() }))).toBe(true);
  });

  it("axisValues reads the requested axis from the attitude block", () => {
    const s = sample({ attitude: attitude() });
    expect(axisValues(s, "yaw")).toEqual({ setpoint: 0.25, measured: 0.2, command: 0.02 });
  });

  it("axisValues falls back to scalar fields for roll and flat-zero for others", () => {
    const s = sample({ setpoint: 1, measured: 0.9, command: 0.1 }); // no attitude
    expect(axisValues(s, "roll")).toEqual({ setpoint: 1, measured: 0.9, command: 0.1 });
    expect(axisValues(s, "pitch")).toEqual({ setpoint: 0, measured: 0, command: 0 });
  });

  it("axisSeries extracts a per-axis measured/setpoint series", () => {
    const buf = [sample({ attitude: attitude() }), sample({ attitude: attitude() })];
    const { measured, setpoint } = axisSeries(buf, "pitch");
    expect(measured).toEqual([-0.3, -0.3]);
    expect(setpoint).toEqual([-0.5, -0.5]);
  });

  it("axesPresent returns all three axes only when attitude is present", () => {
    expect(axesPresent(sample())).toEqual(["roll"]);
    expect(axesPresent(sample({ attitude: attitude() }))).toEqual(["roll", "pitch", "yaw"]);
    expect(axesPresent(undefined)).toEqual(["roll"]);
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
