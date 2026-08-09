// The SoNS swarm made visible: a grid of agents that self-organize (one elected "brain"), cover a
// field via stigmergy, and — new — let old pheromone EVAPORATE so coverage self-heals (the ant
// "useful forgetting"). Judges can hit "Re-run swarm" to watch it re-converge and re-cover.
//
// The cell shading is the LIVE pheromone field (decaying); the ★ marks the current brain; a small
// dot marks each agent. This is the Phase-4 + bee/ant work that the rest of the dashboard doesn't
// otherwise show. Pure presentational + one fetch — no sockets.

import { useCallback, useEffect, useState } from "react";
import { fetchSwarm, fetchQuorum, type SwarmView, type QuorumView } from "../api";

/** Map a pheromone level to a blue-green heat color. 0 = dark (uncovered), high = bright green. */
function heat(level: number, max: number): string {
  if (max <= 0) return "#0b1020";
  const t = Math.min(1, level / max);
  // Interpolate dark navy -> green. Keep it readable on the dark theme.
  const r = Math.round(11 + t * (34 - 11));
  const g = Math.round(16 + t * (222 - 16));
  const b = Math.round(32 + t * (128 - 32));
  return `rgb(${r},${g},${b})`;
}

export function SwarmPanel() {
  const [swarm, setSwarm] = useState<SwarmView | null>(null);
  const [quorum, setQuorum] = useState<QuorumView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    // The swarm snapshot is required; the quorum verdict is best-effort (it needs the MeTTa venv),
    // so a quorum failure just leaves the verdict banner hidden — it never blocks the field view.
    fetchSwarm()
      .then((s) => setSwarm(s))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
    fetchQuorum()
      .then((q) => setQuorum(q))
      .catch(() => setQuorum(null));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (error) {
    return (
      <p data-testid="swarm-error" style={{ color: "#fbbf24" }}>
        No swarm data. Start the API server: <code>cargo run -p nzi-server</code>{" "}
        <span style={{ color: "#6b7fa3" }}>({error})</span>
      </p>
    );
  }

  if (!swarm) {
    return <p style={{ color: "#8aa0c6", fontStyle: "italic" }}>Loading swarm…</p>;
  }

  const maxPheromone = Math.max(1e-6, ...swarm.pheromone);
  const cell = 26; // px per grid cell
  // Build a quick lookup: which agent (if any) sits on each cell, and whether it's the leader.
  const agentAt = new Map<number, { leader: boolean; alive: boolean }>();
  for (const a of swarm.agents) {
    agentAt.set(a.y * swarm.width + a.x, { leader: a.is_leader, alive: a.alive });
  }

  return (
    <div data-testid="swarm-panel">
      <div style={styles.stats}>
        <Chip label="Agents" value={String(swarm.agents.length)} />
        <Chip
          label="Elected brain"
          value={swarm.leader === null ? "none" : `#${swarm.leader}`}
          good={swarm.leader_count === 1}
        />
        <Chip
          label="Field coverage"
          value={`${Math.round(swarm.coverage_fraction * 100)}%`}
          good={swarm.coverage_fraction >= 1}
        />
        <Chip label="Pheromone ρ (decay)" value={swarm.rho.toFixed(2)} />
        <Chip label="Tick" value={String(swarm.tick)} />
      </div>

      {quorum && <QuorumBanner quorum={quorum} />}

      <div style={{ display: "flex", gap: 16, flexWrap: "wrap", alignItems: "flex-start" }}>
        <div
          data-testid="swarm-grid"
          style={{
            display: "grid",
            gridTemplateColumns: `repeat(${swarm.width}, ${cell}px)`,
            gap: 2,
            background: "#070b16",
            padding: 6,
            borderRadius: 8,
            border: "1px solid #23304d",
          }}
        >
          {swarm.pheromone.map((p, i) => {
            const here = agentAt.get(i);
            return (
              <div
                key={i}
                title={`cell ${i % swarm.width},${Math.floor(i / swarm.width)} · pheromone ${p.toFixed(2)}`}
                style={{
                  width: cell,
                  height: cell,
                  background: heat(p, maxPheromone),
                  borderRadius: 3,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: 13,
                  lineHeight: 1,
                }}
              >
                {here?.leader ? (
                  <span style={{ color: "#fde047" }} title="elected brain">
                    ★
                  </span>
                ) : here?.alive ? (
                  <span style={{ color: "#c3d0e6", opacity: 0.7 }}>•</span>
                ) : null}
              </div>
            );
          })}
        </div>

        <div style={{ maxWidth: 260 }}>
          <p style={styles.legend}>
            <span style={{ color: "#fde047" }}>★</span> the self-elected brain (highest-id
            max-consensus) · <span style={{ color: "#c3d0e6" }}>•</span> an agent · cell brightness =
            live <strong>pheromone</strong>, which <em>evaporates</em> each tick so stale coverage
            fades and effort re-flows to gaps.
          </p>
          <button onClick={load} disabled={loading} style={styles.button}>
            {loading ? "running…" : "↻ Re-run swarm"}
          </button>
          <p style={{ ...styles.legend, color: "#6b7fa3" }}>
            Each run spins a fresh 8×8 headless swarm to convergence — no central controller, just
            neighbor-local rules.
          </p>
        </div>
      </div>
    </div>
  );
}

/** The honeybee quorum verdict: does the SLOW MeTTa arbiter commit to the elected brain, or keep
 * scouting? Shown as a banner so a judge sees the DECISION (not just the field). When the MeTTa
 * venv is absent the endpoint returns available:false; we say so honestly rather than faking it. */
function QuorumBanner({ quorum }: { quorum: QuorumView }) {
  const contested = quorum.candidates.reduce((sum, c) => sum + c.inhibition, 0);

  let text: string;
  let color: string;
  if (!quorum.available) {
    text = `Quorum arbiter offline — needs the MeTTa venv${quorum.reason ? ` (${quorum.reason})` : ""}. Candidate evidence still shown below.`;
    color = "#6b7fa3";
  } else if (quorum.verdict === "commit") {
    text = `🐝 Quorum reached (risk=${quorum.risk}) → commit to brain #${quorum.committed_id} at weight ${quorum.weight}. ${quorum.candidates.length} candidate(s), ${contested} cross-inhibition signal(s).`;
    color = "#4ade80";
  } else {
    text = `🐝 Below quorum (risk=${quorum.risk}) → keep scouting (fail-safe, no blind commit). ${quorum.candidates.length} candidate(s), ${contested} cross-inhibition signal(s).`;
    color = "#fbbf24";
  }

  return (
    <p
      data-testid="quorum-banner"
      style={{
        margin: "0 0 12px",
        padding: "8px 12px",
        borderRadius: 8,
        border: `1px solid ${color}`,
        background: "#0b1020",
        color,
        fontSize: 13,
        lineHeight: 1.5,
      }}
    >
      {text}
    </p>
  );
}

function Chip({ label, value, good }: { label: string; value: string; good?: boolean }) {
  return (
    <div style={styles.chip}>
      <div style={styles.chipLabel}>{label}</div>
      <div
        style={{
          ...styles.chipValue,
          color: good === undefined ? "#e6edf7" : good ? "#4ade80" : "#fbbf24",
        }}
      >
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
