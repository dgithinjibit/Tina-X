// GNSS position ingest made visible (G4D-RR bridge #2). Real receiver output (NMEA GGA/RMC, incl.
// multi-constellation `GN` sentences that carry Galileo) is parsed into a track and mapped onto the
// operating grid — the swarm's navigation input. The grid highlights the flown track; the current
// (latest valid) fix is marked. Honest by design: `source` labels provenance and void fixes are
// shown as dropped, never silently placed. Pure presentational + one fetch — no sockets.

import { useCallback, useEffect, useState } from "react";
import { fetchGnss, type GnssView } from "../api";

export function GnssPanel() {
  const [gnss, setGnss] = useState<GnssView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    fetchGnss()
      .then((g) => setGnss(g))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (error) {
    return (
      <p data-testid="gnss-error" style={{ color: "#fbbf24" }}>
        No GNSS data. Start the API server: <code>cargo run -p nzi-server</code>{" "}
        <span style={{ color: "#6b7fa3" }}>({error})</span>
      </p>
    );
  }

  if (!gnss) {
    return <p style={{ color: "#8aa0c6", fontStyle: "italic" }}>Loading GNSS track…</p>;
  }

  const cell = 26; // px per grid cell — same scale as the swarm/flood grids.
  const validFixes = gnss.fixes.filter((f) => f.valid).length;

  // Which cells the track passed through (valid fixes only), and the current cell.
  const onTrack = new Set<number>();
  for (const f of gnss.fixes) {
    if (f.valid && f.cell) onTrack.add(f.cell[1] * gnss.width + f.cell[0]);
  }
  const currentIdx =
    gnss.current_cell !== null ? gnss.current_cell[1] * gnss.width + gnss.current_cell[0] : -1;

  return (
    <div data-testid="gnss-panel">
      <div style={styles.stats}>
        <Chip label="Data source" value={gnss.source === "fixture" ? "offline fixture" : gnss.source} />
        <Chip label="Fixes" value={String(gnss.fixes.length)} />
        <Chip label="Valid fixes" value={`${validFixes}/${gnss.fixes.length}`} good={validFixes > 0} />
        <Chip
          label="Current cell"
          value={gnss.current_cell ? `(${gnss.current_cell[0]},${gnss.current_cell[1]})` : "none"}
          good={gnss.current_cell !== null}
        />
      </div>

      <div style={{ display: "flex", gap: 16, flexWrap: "wrap", alignItems: "flex-start" }}>
        <div
          data-testid="gnss-grid"
          style={{
            display: "grid",
            gridTemplateColumns: `repeat(${gnss.width}, ${cell}px)`,
            gap: 2,
            background: "#070b16",
            padding: 6,
            borderRadius: 8,
            border: "1px solid #23304d",
          }}
        >
          {Array.from({ length: gnss.width * gnss.height }, (_, i) => {
            const isCurrent = i === currentIdx;
            const isTrack = onTrack.has(i);
            const bg = isCurrent ? "#38bdf8" : isTrack ? "#164e63" : "#0b1020";
            return (
              <div
                key={i}
                title={`cell ${i % gnss.width},${Math.floor(i / gnss.width)}`}
                style={{
                  width: cell,
                  height: cell,
                  background: bg,
                  borderRadius: 3,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: 13,
                }}
              >
                {isCurrent ? <span style={{ color: "#05233b" }} title="current fix">◉</span> : null}
              </div>
            );
          })}
        </div>

        <div style={{ maxWidth: 300 }}>
          <p style={styles.legend}>
            <span style={{ color: "#38bdf8" }}>◉</span> current fix ·{" "}
            <span style={{ color: "#164e63" }}>▓</span> flown track. Positions are parsed from{" "}
            <strong>NMEA</strong> GGA/RMC sentences — the multi-constellation <code>GN</code> form
            carries <strong>Galileo</strong>. Void fixes are dropped, not placed. RTKLIB (RTK/PPP)
            output is NMEA too, so it ingests through the same path.
          </p>
          <button onClick={load} disabled={loading} style={styles.button}>
            {loading ? "loading…" : "↻ Re-fetch track"}
          </button>
        </div>
      </div>
    </div>
  );
}

function Chip({ label, value, good }: { label: string; value: string; good?: boolean }) {
  return (
    <div style={styles.chip}>
      <div style={styles.chipLabel}>{label}</div>
      <div style={{ ...styles.chipValue, color: good === undefined ? "#e6edf7" : good ? "#4ade80" : "#fbbf24" }}>
        {value}
      </div>
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
