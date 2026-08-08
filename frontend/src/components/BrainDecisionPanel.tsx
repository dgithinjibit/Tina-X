// The "verifiability moat" made visible: a list of the SLOW symbolic brain's recent decisions,
// each with the MeTTa query it ran, the atoms it got back, and whether verification approved it.
//
// For a junior dev: this is a pure presentational component — it just renders whatever
// decisions the `useTelemetry` hook collected. No fetching or sockets here.

import type { BrainDecision } from "../useTelemetry";

interface Props {
  decisions: BrainDecision[];
}

export function BrainDecisionPanel({ decisions }: Props) {
  if (decisions.length === 0) {
    // The slow brain is low-rate and Phase 2 populates it richly; until then, say so honestly
    // rather than showing a fake decision.
    return (
      <p data-testid="brain-empty" style={{ color: "#8aa0c6", fontStyle: "italic" }}>
        No brain decisions yet — the slow MeTTa brain streams here once Phase 2 wires it in.
      </p>
    );
  }

  return (
    <ul data-testid="brain-list" style={{ listStyle: "none", padding: 0, margin: 0 }}>
      {decisions.map((d) => (
        <li
          key={d.id}
          data-testid="brain-item"
          style={{
            border: "1px solid #23304d",
            borderRadius: 8,
            padding: "8px 10px",
            marginBottom: 8,
            background: "#0b1020",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
            <code style={{ color: "#93c5fd" }}>{d.query}</code>
            {/* A verified decision is the whole point of the moat — flag it prominently. */}
            <span
              data-testid="brain-verified"
              style={{ color: d.verified ? "#4ade80" : "#f87171", fontWeight: 600 }}
            >
              {d.verified ? "✓ verified" : "✗ unverified"}
            </span>
          </div>
          <div style={{ color: "#c8d3e6", marginTop: 4, fontSize: 13 }}>
            → {d.results.length ? d.results.join(", ") : "(no results)"}
          </div>
        </li>
      ))}
    </ul>
  );
}
