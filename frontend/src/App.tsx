// The Project Nzi landing + mission-control page.
//
// Structure (top to bottom):
//   1. Hero        — what Nzi is, for a judge who has 30 seconds.
//   2. Live reflex — real telemetry from the Rust server (proves the control loop works).
//   3. How it works — the two-rate brain, in plain words.
//   4. BOM         — the IoT hardware, because this is a physical project.
//
// Data comes from the Rust server (rust-server/). If the server isn't running, the page still
// renders (landing + how-it-works) and shows friendly "start the server" hints.

import { useEffect, useState } from "react";
import { fetchBom, type AxisName, type Bom } from "./api";
import { axesPresent, computeStats } from "./telemetry";
import { useTelemetry } from "./useTelemetry";
import { ReflexChart } from "./components/ReflexChart";
import { BrainDecisionPanel } from "./components/BrainDecisionPanel";
import { BomTable } from "./components/BomTable";

// Color per body axis so the three traces are easy to tell apart at a glance.
const AXIS_COLORS: Record<AxisName, string> = {
  roll: "#4ade80",
  pitch: "#fbbf24",
  yaw: "#c084fc",
};

export function App() {
  const { samples, decisions, conn } = useTelemetry();
  const [bom, setBom] = useState<Bom | null>(null);
  const [bomError, setBomError] = useState<string | null>(null);

  useEffect(() => {
    fetchBom()
      .then(setBom)
      .catch((e: unknown) => setBomError(e instanceof Error ? e.message : String(e)));
  }, []);

  const stats = computeStats(samples);
  const budgetOk = stats.allWithinBudget;
  // Which axes to chart: all three when the sim streams 3-axis data, else just roll.
  const axes = axesPresent(samples[samples.length - 1]);

  return (
    <main style={styles.page}>
      {/* 1. HERO ------------------------------------------------------------------ */}
      <header style={styles.hero}>
        <h1 style={styles.title}>🪰 Project Nzi</h1>
        <p style={styles.tagline}>
          Bio-inspired autonomous robot swarms for a <strong>good life 2030</strong>. We borrow the
          fly&apos;s biology — fast reflexes, optic-flow vision, collision tolerance, swarm
          behavior — to build cheap, decentralized agents for precision agriculture and
          harsh-weather early warning. <em>Nzi</em> means &quot;fly&quot; in Swahili.
        </p>
        <div style={styles.badges}>
          <Badge label="Rust core" />
          <Badge label="MeTTa symbolic brain" />
          <Badge label="Python ML" />
          <Badge label="React + TS" />
          <Badge label="IoT / edge" />
        </div>
      </header>

      {/* 2. LIVE REFLEX ----------------------------------------------------------- */}
      <section style={styles.section}>
        <h2 style={styles.h2}>Live reflex telemetry</h2>
        <p style={styles.muted}>
          A real control loop (the same code that would fly the robot) running on the Rust server,
          streamed here. Each chart is one body axis — colored line = measured rate, blue dashed =
          target. The fly stabilizes in ~13 ms; our loop runs far under that.
        </p>

        <div style={styles.statsRow}>
          <Stat label="Connection" value={conn} good={conn === "open"} />
          <Stat label="Samples" value={String(stats.count)} />
          <Stat label="Max latency" value={`${stats.maxLatencyUs} µs`} good={budgetOk} />
          <Stat label="Avg latency" value={`${stats.avgLatencyUs.toFixed(1)} µs`} />
          <Stat label="Tracking error" value={stats.lastError.toFixed(3)} good={stats.lastError < 0.05} />
          <Stat label="Within 13 ms budget" value={budgetOk ? "yes" : "NO"} good={budgetOk} />
        </div>

        <div style={styles.axisGrid}>
          {axes.map((axis) => (
            <figure key={axis} style={styles.axisFigure}>
              <figcaption style={styles.axisCaption}>
                <span style={{ color: AXIS_COLORS[axis] }}>●</span> {axis}
              </figcaption>
              <ReflexChart
                samples={samples}
                axis={axis}
                measuredColor={AXIS_COLORS[axis]}
                width={280}
                height={120}
              />
            </figure>
          ))}
        </div>

        {conn !== "open" && (
          <p style={styles.hint}>
            No live data yet. Start the API server: <code>cargo run -p nzi-server</code>
          </p>
        )}
      </section>

      {/* 2b. SLOW BRAIN ----------------------------------------------------------- */}
      <section style={styles.section}>
        <h2 style={styles.h2}>Slow symbolic brain — verified decisions</h2>
        <p style={styles.muted}>
          The MeTTa brain&apos;s recent decisions, each with the query it ran and whether the
          verification layer approved it. This is the &quot;verifiability moat&quot;: judges can see
          the agent <em>reason</em>, not just act.
        </p>
        <BrainDecisionPanel decisions={decisions} />
      </section>

      {/* 3. HOW IT WORKS ---------------------------------------------------------- */}
      <section style={styles.section}>
        <h2 style={styles.h2}>How it works — the two-rate brain</h2>
        <div style={styles.cards}>
          <Card title="Fast reflex loop (µs)">
            A delayed-PD stabilizer inspired by the fly&apos;s halteres. Runs at 500 Hz, well under
            the ~13 ms biological budget. This keeps the agent stable no matter what the slow brain
            is thinking.
          </Card>
          <Card title="Slow symbolic brain (ms)">
            MeTTa / Hyperon reasons over the world, coordinates the swarm, and <em>verifies</em> its
            own decisions — our defense against &quot;confident but wrong&quot; agent behavior. It
            never runs inside the reflex path (proven by our benchmarks).
          </Card>
          <Card title="Swarm nervous system">
            Agents self-organize into runtime hierarchies (SoNS) using neighbor-local comms — like a
            fly swarm. Build one reliable bot first, then scale to a fleet.
          </Card>
        </div>
      </section>

      {/* 4. BOM ------------------------------------------------------------------- */}
      <section style={styles.section}>
        <h2 style={styles.h2}>Bill of materials — one Nzi agent</h2>
        <p style={styles.muted}>
          This is real hardware. Each part traces to a fly trait or a research finding, and the
          whole agent is deliberately cheap so a swarm is affordable.
        </p>
        <BomTable bom={bom} error={bomError} />
      </section>

      <footer style={styles.footer}>
        Project Nzi · MIT · research-backed &amp; benchmarked · not war — a good life.
      </footer>
    </main>
  );
}

// --- tiny presentational helpers (kept in-file to stay dependency-light) ---------------------

function Badge({ label }: { label: string }) {
  return <span style={styles.badge}>{label}</span>;
}

function Stat({ label, value, good }: { label: string; value: string; good?: boolean }) {
  return (
    <div style={styles.stat}>
      <div style={styles.statLabel}>{label}</div>
      <div style={{ ...styles.statValue, color: good === undefined ? "#e6edf7" : good ? "#4ade80" : "#f87171" }}>
        {value}
      </div>
    </div>
  );
}

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={styles.card}>
      <h3 style={styles.cardTitle}>{title}</h3>
      <p style={styles.muted}>{children}</p>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { maxWidth: 900, margin: "0 auto", padding: "24px 16px 64px", color: "#e6edf7", fontFamily: "system-ui, sans-serif", background: "#070b16", minHeight: "100vh" },
  hero: { padding: "24px 0" },
  title: { fontSize: "2.4rem", margin: 0 },
  tagline: { fontSize: "1.1rem", color: "#c3d0e6", lineHeight: 1.5, maxWidth: 720 },
  badges: { display: "flex", gap: 8, flexWrap: "wrap", marginTop: 12 },
  badge: { background: "#16203a", border: "1px solid #23304d", borderRadius: 999, padding: "4px 12px", fontSize: "0.85rem" },
  section: { marginTop: 32 },
  h2: { fontSize: "1.4rem", borderBottom: "1px solid #23304d", paddingBottom: 6 },
  muted: { color: "#9fb3d1", lineHeight: 1.5 },
  hint: { color: "#fbbf24", marginTop: 8 },
  statsRow: { display: "flex", gap: 10, flexWrap: "wrap", margin: "12px 0" },
  axisGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: 12 },
  axisFigure: { margin: 0 },
  axisCaption: { textTransform: "capitalize", color: "#c3d0e6", fontWeight: 600, marginBottom: 4 },
  stat: { background: "#0b1020", border: "1px solid #23304d", borderRadius: 8, padding: "8px 12px", minWidth: 110 },
  statLabel: { fontSize: "0.75rem", color: "#6b7fa3" },
  statValue: { fontSize: "1.1rem", fontWeight: 700 },
  cards: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12, marginTop: 12 },
  card: { background: "#0b1020", border: "1px solid #23304d", borderRadius: 8, padding: 16 },
  cardTitle: { margin: "0 0 8px", fontSize: "1.05rem" },
  footer: { marginTop: 48, color: "#6b7fa3", fontSize: "0.85rem", textAlign: "center" },
};
