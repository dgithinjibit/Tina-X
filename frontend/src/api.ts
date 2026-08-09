// API layer: the TypeScript mirror of the Rust telemetry/BOM contract.
//
// For a junior dev: these types MUST match the Rust structs in
//   rust-core/src/telemetry.rs  and  rust-server/src/bom.rs
// If you change a field name on one side, change it on both — the tests here guard the shapes.

/** Base URL of the Rust mission-control API. Overridable via Vite env for deploys. */
export const API_BASE: string =
  (import.meta as { env?: Record<string, string> }).env?.VITE_NZI_API ??
  "http://127.0.0.1:8080";

/** WebSocket URL for live telemetry (derived from API_BASE). */
export function wsUrl(base: string = API_BASE): string {
  // http -> ws, https -> wss.
  return base.replace(/^http/, "ws") + "/ws";
}

/** A roll/pitch/yaw triple (mirror of Rust `Axis3`). */
export interface Axis3 {
  roll: number;
  pitch: number;
  yaw: number;
}

/** Per-axis setpoint/measured/command for one step (mirror of Rust `AttitudeSample`). */
export interface AttitudeSample {
  setpoint: Axis3;
  measured: Axis3;
  command: Axis3;
}

/** The three body axes, in a fixed order for iterating/rendering. */
export const AXES = ["roll", "pitch", "yaw"] as const;
export type AxisName = (typeof AXES)[number];

/** One reflex-loop sample (mirror of Rust `ReflexSample`).
 *
 * `attitude` is OPTIONAL: single-axis runs omit it (the scalar fields are the roll axis),
 * multi-axis runs include the full 3-axis detail. Matches the serde `skip_serializing_if`. */
export interface ReflexSample {
  step: number;
  setpoint: number;
  measured: number;
  command: number;
  latency_us: number;
  attitude?: AttitudeSample;
}

/** A WebSocket telemetry message (mirror of Rust tagged enum `TelemetryMsg`). */
export type TelemetryMsg =
  | ({ type: "reflex" } & ReflexSample)
  | { type: "decision"; id: number; query: string; results: string[]; verified: boolean }
  | {
      type: "status";
      agent_id: string;
      healthy: boolean;
      last_latency_us: number;
      last_error: number;
    };

/** One bill-of-materials line item (mirror of Rust `BomItem`). */
export interface BomItem {
  name: string;
  purpose: string;
  rationale: string;
  est_cost_usd: number;
  qty: number;
}

/** The full BOM (mirror of Rust `Bom`). */
export interface Bom {
  items: BomItem[];
  total_est_cost_usd: number;
}

/** Fetch the bill of materials from the server. */
export async function fetchBom(base: string = API_BASE): Promise<Bom> {
  const res = await fetch(`${base}/api/bom`);
  if (!res.ok) throw new Error(`BOM request failed: ${res.status}`);
  return (await res.json()) as Bom;
}

/** One agent in a swarm snapshot (mirror of Rust `AgentView`). */
export interface AgentView {
  id: number;
  x: number;
  y: number;
  alive: boolean;
  is_leader: boolean;
  distance_to_leader: number;
  covering: boolean;
}

/** A snapshot of the SoNS swarm + stigmergy field (mirror of Rust `SwarmView`).
 *
 * `pheromone` and `coverage` are ROW-MAJOR grids of length width*height: cell (x,y) is at index
 * y*width + x. `pheromone` decays each tick (the ant "useful forgetting"); `coverage` is cumulative. */
export interface SwarmView {
  width: number;
  height: number;
  tick: number;
  leader: number | null;
  leader_count: number;
  coverage_fraction: number;
  rho: number;
  agents: AgentView[];
  pheromone: number[];
  coverage: number[];
}

/** Fetch a swarm snapshot from the server (runs the headless SoNS swarm to convergence). */
export async function fetchSwarm(base: string = API_BASE): Promise<SwarmView> {
  const res = await fetch(`${base}/api/swarm`);
  if (!res.ok) throw new Error(`Swarm request failed: ${res.status}`);
  return (await res.json()) as SwarmView;
}

/** One leader-candidate's neighbor-local tally (mirror of Rust `CandidateView`). */
export interface CandidateView {
  id: number;
  support: number;
  inhibition: number;
}

/** The honeybee quorum verdict over the swarm's leader candidates (mirror of Rust `QuorumView`).
 *
 * When `available` is false (MeTTa venv absent), `reason` explains why and `verdict` is omitted —
 * the candidate evidence is still present so the UI can show the tallies honestly. */
export interface QuorumView {
  available: boolean;
  reason?: string;
  risk: string;
  candidates: CandidateView[];
  verdict?: "commit" | "scout";
  committed_id?: number;
  weight?: number;
}

/** Fetch the honeybee quorum verdict for the swarm's elected brain (runs the SLOW MeTTa arbiter). */
export async function fetchQuorum(base: string = API_BASE): Promise<QuorumView> {
  const res = await fetch(`${base}/api/quorum`);
  if (!res.ok) throw new Error(`Quorum request failed: ${res.status}`);
  return (await res.json()) as QuorumView;
}

/** One river reach in the flood forecast (mirror of Rust `ReachView`). */
export interface ReachView {
  reach_id: number;
  x: number;
  y: number;
  forecast_cms: number;
  threshold_cms: number;
  /** Normalized flood risk in [0,1] (forecast vs. warning threshold). */
  risk: number;
}

/** The GEOGLOWS flood early-warning overlay driving the swarm (mirror of Rust `FloodView`).
 *
 * `risk` is a ROW-MAJOR grid of length width*height in [0,1] (cell (x,y) at y*width + x) — the same
 * layout as `SwarmView.pheromone`/`coverage`, so the UI can overlay them. `source` is honest data
 * provenance: `"fixture"` (offline demo scenario) or `"geoglows-live"` (a real fetch). This is FLOOD
 * streamflow forecasting, NOT earthquake prediction — see docs/research/g4drr-gnss-eo-bridge.md. */
export interface FloodView {
  width: number;
  height: number;
  source: string;
  risk: number[];
  mean_risk: number;
  reaches: ReachView[];
  /** Fraction of the field the swarm covered while biased by this flood overlay, in [0,1]. */
  coverage_fraction: number;
}

/** Fetch the flood early-warning overlay (GEOGLOWS-shaped forecast mapped to the swarm hazard field). */
export async function fetchFlood(base: string = API_BASE): Promise<FloodView> {
  const res = await fetch(`${base}/api/flood`);
  if (!res.ok) throw new Error(`Flood request failed: ${res.status}`);
  return (await res.json()) as FloodView;
}

/** One parsed GNSS fix mapped onto the grid (mirror of Rust `GnssFixView`). `cell` is null when the
 * fix is invalid/void (not placed on the grid). */
export interface GnssFixView {
  lat: number;
  lon: number;
  valid: boolean;
  cell: [number, number] | null;
}

/** A GNSS position track ingested from NMEA and mapped onto the grid (mirror of Rust `GnssView`).
 *
 * This is G4D-RR bridge #2: real receiver output (NMEA GGA/RMC, incl. multi-constellation `GN`
 * sentences that carry Galileo) becomes swarm navigation input. `source` is honest provenance
 * (`"fixture"` offline track today). `current_cell` is where the swarm would navigate now (latest
 * valid fix), or null if no fix is valid. */
export interface GnssView {
  width: number;
  height: number;
  source: string;
  fixes: GnssFixView[];
  current_cell: [number, number] | null;
}

/** Fetch the GNSS position track (NMEA fixes parsed and mapped onto the operating grid). */
export async function fetchGnss(base: string = API_BASE): Promise<GnssView> {
  const res = await fetch(`${base}/api/gnss`);
  if (!res.ok) throw new Error(`GNSS request failed: ${res.status}`);
  return (await res.json()) as GnssView;
}

/** The fly-derived reflex latency budget in microseconds (mirror of Rust REFLEX_BUDGET_US). */
export const REFLEX_BUDGET_US = 13_000;
