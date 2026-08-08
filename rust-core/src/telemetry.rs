//! Telemetry: the structured data an Nzi agent emits so humans (and the dashboard) can see
//! what it's doing. These types are the CONTRACT between the agent and the frontend.
//!
//! # For a junior dev
//! We keep these types in `nzi-core` (not the server) because the *agent* produces them. They
//! derive `Serialize`/`Deserialize` so they become JSON automatically for the WebSocket/HTTP API.
//! Keep them small and stable — the React app depends on their field names.

use serde::{Deserialize, Serialize};

/// One sample from the fast reflex loop (see `crate::reflex`). Emitted many times per second.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReflexSample {
    /// Monotonic step counter since the agent started.
    pub step: u64,
    /// Desired attitude rate (what the slow brain asked for).
    pub setpoint: f32,
    /// Measured attitude rate (from the rate gyro / halteres analogue).
    pub measured: f32,
    /// Actuator command the reflex loop produced this step.
    pub command: f32,
    /// How long this reflex step took, in microseconds (must stay under REFLEX_BUDGET_US).
    pub latency_us: u32,
}

/// A decision made by the SLOW symbolic brain (MeTTa), with its justification — this is the
/// "verifiability moat" made visible: judges can SEE the agent reason, not just act.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrainDecision {
    /// Monotonic decision counter.
    pub id: u64,
    /// The MeTTa query that was asked (human-readable).
    pub query: String,
    /// The result atoms MeTTa returned.
    pub results: Vec<String>,
    /// Whether the verification layer approved acting on this (Phase 2 will populate richly).
    pub verified: bool,
}

/// A coarse health/status snapshot of one agent, for the dashboard's agent list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent identifier, e.g. "nzi-001".
    pub agent_id: String,
    /// True while the reflex loop is stable and within budget.
    pub healthy: bool,
    /// Last reflex latency seen (µs) — quick at-a-glance signal for the budget.
    pub last_latency_us: u32,
    /// Most recent absolute tracking error (|setpoint - measured|).
    pub last_error: f32,
}

/// The top-level message the server pushes to the frontend over WebSocket.
///
/// A tagged enum so the React side can `switch` on `type` — serde emits e.g.
/// `{"type":"reflex","step":..,..}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TelemetryMsg {
    Reflex(ReflexSample),
    Decision(BrainDecision),
    Status(AgentStatus),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflex_sample_round_trips_json() {
        let s = ReflexSample { step: 7, setpoint: 1.0, measured: 0.9, command: 0.1, latency_us: 12 };
        let json = serde_json::to_string(&s).unwrap();
        let back: ReflexSample = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn telemetry_msg_is_tagged_for_the_frontend() {
        // The React app switches on the "type" tag; lock that contract with a test.
        let msg = TelemetryMsg::Reflex(ReflexSample {
            step: 1, setpoint: 0.0, measured: 0.0, command: 0.0, latency_us: 3,
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"reflex\""), "got: {json}");
    }

    #[test]
    fn decision_carries_query_and_results() {
        let d = BrainDecision {
            id: 1,
            query: "!(safe? 8)".into(),
            results: vec!["True".into()],
            verified: true,
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: BrainDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
        assert!(json.contains("\"verified\":true"));
    }
}
