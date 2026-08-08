// A dependency-free SVG sparkline showing the reflex loop tracking its setpoint.
//
// For a junior dev: we draw two polylines (setpoint vs measured) by mapping each sample to an
// (x, y) pixel. No chart library — keeps the bundle tiny and the logic inspectable.

import type { AxisName, ReflexSample } from "../api";
import { axisSeries } from "../telemetry";

interface Props {
  samples: ReflexSample[];
  /** Which body axis to plot. Defaults to "roll" (the primary/legacy scalar axis). */
  axis?: AxisName;
  /** Color of the measured line — lets the caller color-code the three axes. */
  measuredColor?: string;
  width?: number;
  height?: number;
}

/** Map a series of numbers to an SVG polyline "points" string within [0,width]x[0,height]. */
function toPolyline(values: number[], width: number, height: number, min: number, max: number): string {
  if (values.length === 0) return "";
  const span = max - min || 1; // avoid divide-by-zero when the signal is flat
  const stepX = width / Math.max(1, values.length - 1);
  return values
    .map((v, i) => {
      const x = i * stepX;
      // Invert Y because SVG's origin is top-left.
      const y = height - ((v - min) / span) * height;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function ReflexChart({
  samples,
  axis = "roll",
  measuredColor = "#4ade80",
  width = 480,
  height = 140,
}: Props) {
  // Pull the chosen axis's series (falls back to the scalar/roll fields for single-axis data).
  const { measured, setpoint: setpoints } = axisSeries(samples, axis);

  // Shared vertical scale across both lines so they're comparable.
  const all = [...setpoints, ...measured];
  const min = all.length ? Math.min(...all) : 0;
  const max = all.length ? Math.max(...all) : 1;

  return (
    <svg
      role="img"
      aria-label={`Reflex loop (${axis}): setpoint vs measured`}
      width={width}
      height={height}
      style={{ background: "#0b1020", borderRadius: 8, border: "1px solid #23304d" }}
    >
      {/* measured (what the agent is actually doing) */}
      <polyline
        data-testid="measured-line"
        fill="none"
        stroke={measuredColor}
        strokeWidth={2}
        points={toPolyline(measured, width, height, min, max)}
      />
      {/* setpoint (what the slow brain asked for) */}
      <polyline
        data-testid="setpoint-line"
        fill="none"
        stroke="#60a5fa"
        strokeWidth={1.5}
        strokeDasharray="4 3"
        points={toPolyline(setpoints, width, height, min, max)}
      />
    </svg>
  );
}
