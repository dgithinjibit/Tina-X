// Pure telemetry helpers — no React, no DOM — so they are trivially unit-testable.

import { AXES, REFLEX_BUDGET_US, type AxisName, type ReflexSample } from "./api";

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

/** Absolute tracking error for a sample (|setpoint - measured|).
 *
 * If the sample carries 3-axis detail, this is the WORST-axis error (matching the Rust sim's
 * `last_error`); otherwise it falls back to the single scalar axis. */
export function trackingError(sample: ReflexSample): number {
  if (sample.attitude) {
    const { setpoint, measured } = sample.attitude;
    return Math.max(
      Math.abs(setpoint.roll - measured.roll),
      Math.abs(setpoint.pitch - measured.pitch),
      Math.abs(setpoint.yaw - measured.yaw),
    );
  }
  return Math.abs(sample.setpoint - sample.measured);
}

/** Does this sample carry the richer 3-axis attitude detail? */
export function hasAttitude(sample: ReflexSample): boolean {
  return sample.attitude !== undefined;
}

/** One axis's setpoint/measured/command for a sample. Falls back to the scalar (roll) fields
 * when no 3-axis detail is present, so callers can treat every sample uniformly. */
export function axisValues(
  sample: ReflexSample,
  axis: AxisName,
): { setpoint: number; measured: number; command: number } {
  if (sample.attitude) {
    return {
      setpoint: sample.attitude.setpoint[axis],
      measured: sample.attitude.measured[axis],
      command: sample.attitude.command[axis],
    };
  }
  // No 3-axis detail: only the primary (roll) axis is meaningful; others read as flat zero.
  if (axis === "roll") {
    return { setpoint: sample.setpoint, measured: sample.measured, command: sample.command };
  }
  return { setpoint: 0, measured: 0, command: 0 };
}

/** Extract the measured-vs-setpoint series for one axis across a window, for charting. */
export function axisSeries(
  buffer: ReflexSample[],
  axis: AxisName,
): { measured: number[]; setpoint: number[] } {
  const measured: number[] = [];
  const setpoint: number[] = [];
  for (const s of buffer) {
    const v = axisValues(s, axis);
    measured.push(v.measured);
    setpoint.push(v.setpoint);
  }
  return { measured, setpoint };
}

/** The list of axes actually present in a sample: all three if 3-axis, else just roll. */
export function axesPresent(sample: ReflexSample | undefined): readonly AxisName[] {
  return sample?.attitude ? AXES : (["roll"] as const);
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
