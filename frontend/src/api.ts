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

/** One reflex-loop sample (mirror of Rust `ReflexSample`). */
export interface ReflexSample {
  step: number;
  setpoint: number;
  measured: number;
  command: number;
  latency_us: number;
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

/** The fly-derived reflex latency budget in microseconds (mirror of Rust REFLEX_BUDGET_US). */
export const REFLEX_BUDGET_US = 13_000;
