// The GEOGLOWS flood early-warning overlay made visible (G4D-RR bridge #6). A free, no-auth river
// streamflow forecast becomes a flood-risk field that the SoNS swarm biases its coverage toward —
// so effort concentrates on the flooded corridor. The red heat map is the forecast risk; the chips
// summarize the threat and the coverage the swarm achieves UNDER it. Judges can hit "Re-run" to
// re-fetch. Pure presentational + one fetch — no sockets. Honest by design: `source` labels
// provenance (fixture vs. live) and this is FLOOD forecasting, not earthquake prediction.

import { useCallback, useEffect, useState } from "react";
import { fetchFlood, type FloodView } from "../api";

/** Map a flood-risk level [0,1] to a dark→red heat color (readable on the dark theme). */
function floodHeat(risk: number): string {
  const t = Math.min(1, Math.max(0, risk));
  // Dark navy (safe) -> bright red (flooded).
  const r = Math.round(11 + t * (239 - 11));
  const g = Math.round(16 + t * (68 - 16));
  const b = Math.round(32 + t * (68 - 32));
  return `rgb(${r},${g},${b})`;
}

export function FloodPanel() {
  const [flood, setFlood] = useState<FloodView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    fetchFlood()
      .then((f) => setFlood(f))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (error) {
    return (
      <p data-testid="flood-error" style={{ color: "#fbbf24" }}>
        No flood data. Start the API server: <code>cargo run -p tina-server</code>{" "}
        <span style={{ color: "#6b7fa3" }}>({error})</span>
      </p>
    );
  }

  if (!flood) {
    return <p style={{ color: "#8aa0c6", fontStyle: "italic" }}>Loading flood forecast…</p>;
  }

  const cell = 26; // px per grid cell — same scale as the swarm grid so they read as one area.
  const atRisk = flood.risk.filter((r) => r >= 0.5).length;

  return (
    <div data-testid="flood-panel">
      <div style={styles.stats}>
        <Chip
          label="Data source"
          value={flood.source === "fixture" ? "offline fixture" : flood.source}
        />
        <Chip
          label="Mean flood risk"
          value={`${Math.round(flood.mean_risk * 100)}%`}
          bad={flood.mean_risk >= 0.3}
        />
        <Chip label="Cells in flood" value={`${atRisk}/${flood.risk.length}`} bad={atRisk > 0} />
        <Chip label="River reaches" value={String(flood.reaches.length)} />
        <Chip
          label="Swarm coverage"
          value={`${Math.round(flood.coverage_fraction * 100)}%`}
          good={flood.coverage_fraction >= 1}
        />
      </div>

      <div style={{ display: "flex", gap: 16, flexWrap: "wrap", alignItems: "flex-start" }}>
        <div
          data-testid="flood-grid"
          style={{
            display: "grid",
            gridTemplateColumns: `repeat(${flood.width}, ${cell}px)`,
            gap: 2,
            background: "#070b16",
            padding: 6,
            borderRadius: 8,
            border: "1px solid #23304d",
          }}
        >
          {flood.risk.map((r, i) => (
            <div
              key={i}
              title={`cell ${i % flood.width},${Math.floor(i / flood.width)} · flood risk ${(r * 100).toFixed(0)}%`}
              style={{
                width: cell,
                height: cell,
                background: floodHeat(r),
                borderRadius: 3,
              }}
            />
          ))}
        </div>

        <div style={{ maxWidth: 300 }}>
          <p style={styles.legend}>
            Red = forecast <strong>flood risk</strong> from{" "}
            <a
              href="https://geoglows.ecmwf.int/documentation"
              target="_blank"
              rel="noreferrer"
              style={{ color: "#7aa2f7" }}
            >
              GEOGLOWS ECMWF Streamflow
            </a>{" "}
            (free, no-auth). The swarm biases its coverage toward these cells — the disaster pulls
            the effort. This is <em>flood</em> forecasting, not earthquake prediction.
          </p>
          <button onClick={load} disabled={loading} style={styles.button}>
            {loading ? "loading…" : "↻ Re-run forecast"}
          </button>
        </div>
      </div>
    </div>
  );
}

function Chip({
  label,
  value,
  good,
  bad,
}: {
  label: string;
  value: string;
  good?: boolean;
  bad?: boolean;
}) {
  const color = bad ? "#f87171" : good ? "#4ade80" : "#e6edf7";
  return (
    <div style={styles.chip}>
      <div style={styles.chipLabel}>{label}</div>
      <div style={{ ...styles.chipValue, color }}>{value}</div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  stats: { display: "flex", gap: 10, flexWrap: "wrap", margin: "12px 0" },
  chip: { background: "#0b1020", border: "1px solid #23304d", borderRadius: 8, padding: "8px 12px", minWidth: 110 },
  chipLabel: { fontSize: "0.75rem", color: "#6b7fa3" },
  chipValue: { fontSize: "1.1rem", fontWeight: 700 },
  legend: { color: "#9fb3d1", fontSize: 13, lineHeight: 1.5 },
  button: { background: "#16203a", color: "#e6edf7", border: "1px solid #23304d", borderRadius: 8, padding: "8px 14px", cursor: "pointer", fontSize: 14 },
};
