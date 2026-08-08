// Pure telemetry helpers — no React, no DOM — so they are trivially unit-testable.

import { REFLEX_BUDGET_US, type ReflexSample } from "./api";

/** Keep only the most recent `max` samples (a rolling window for the live chart). */
export function pushWindow(
  buffer: ReflexSample[],
  sample: ReflexSample,
  max = 120,
): ReflexSample[] {
  const next = [...buffer, sample];
  return next.length > max ? next.slice(next.length - max) : next;
}

/** True if a reflex step stayed within the fly-derived latency budget. */
export function withinBudget(sample: ReflexSample): boolean {
  return sample.latency_us <= REFLEX_BUDGET_US;
}

/** Absolute tracking error for a sample (|setpoint - measured|). */
export function trackingError(sample: ReflexSample): number {
  return Math.abs(sample.setpoint - sample.measured);
}

/** Simple stats over a window, for the dashboard header. */
export interface ReflexStats {
  count: number;
  maxLatencyUs: number;
  avgLatencyUs: number;
  lastError: number;
  allWithinBudget: boolean;
}

export function computeStats(buffer: ReflexSample[]): ReflexStats {
  if (buffer.length === 0) {
    return { count: 0, maxLatencyUs: 0, avgLatencyUs: 0, lastError: 0, allWithinBudget: true };
  }
  let maxLatency = 0;
  let sumLatency = 0;
  let allOk = true;
  for (const s of buffer) {
    maxLatency = Math.max(maxLatency, s.latency_us);
    sumLatency += s.latency_us;
    if (!withinBudget(s)) allOk = false;
  }
  const last = buffer[buffer.length - 1];
  return {
    count: buffer.length,
    maxLatencyUs: maxLatency,
    avgLatencyUs: sumLatency / buffer.length,
    lastError: trackingError(last),
    allWithinBudget: allOk,
  };
}
