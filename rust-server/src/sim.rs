//! A tiny live simulation that drives the reflex loop and produces telemetry.
//!
//! # For a junior dev
//! The dashboard needs *something* to show. Until we have real hardware (Phase 6) or the Unity
//! sim (Phase 1), this module runs the SAME reflex stabilizer from `nzi-core` against a toy
//! plant and emits [`ReflexSample`]s — so judges see a real, running control loop, not a mockup.
//!
//! This is pure logic (no async, no HTTP) so it is easy to unit-test. The server layer just
//! calls [`AgentSim::tick`] on a timer and forwards the samples.

use nzi_core::reflex::{PdGains, ReflexStabilizer};
use nzi_core::telemetry::{AgentStatus, ReflexSample};
use nzi_core::REFLEX_BUDGET_US;
use std::time::Instant;

/// One simulated Nzi agent: a reflex stabilizer + a toy rate-integrator plant.
pub struct AgentSim {
    agent_id: String,
    stab: ReflexStabilizer,
    setpoint: f32,
    measured: f32,
    dt: f32,
    step: u64,
    last_latency_us: u32,
    last_error: f32,
}

impl AgentSim {
    /// Create a sim for `agent_id` at control period `dt` seconds (e.g. 1/500 for 500 Hz).
    pub fn new(agent_id: impl Into<String>, dt: f32) -> Self {
        Self {
            agent_id: agent_id.into(),
            stab: ReflexStabilizer::new(PdGains::default()),
            setpoint: 1.0,
            measured: 0.0,
            dt,
            step: 0,
            last_latency_us: 0,
            last_error: 0.0,
        }
    }

    /// Change the target rate the agent is trying to hold (the "slow brain" would set this).
    pub fn set_setpoint(&mut self, setpoint: f32) {
        self.setpoint = setpoint;
    }

    /// Advance the simulation one control step and return the telemetry sample.
    ///
    /// We time only the controller call (not the plant integration) so `latency_us` reflects
    /// the real reflex cost — the number the dashboard compares against the 13 ms budget.
    pub fn tick(&mut self) -> ReflexSample {
        let t0 = Instant::now();
        let command = self.stab.step(self.setpoint, self.measured, self.dt);
        let latency_us = t0.elapsed().as_micros() as u32;

        // Integrate the toy rate-integrator plant OUTSIDE the timed region (matches nzi-core).
        self.measured += command * self.dt;

        self.step += 1;
        self.last_latency_us = latency_us;
        self.last_error = (self.setpoint - self.measured).abs();

        ReflexSample {
            step: self.step,
            setpoint: self.setpoint,
            measured: self.measured,
            command,
            latency_us,
        }
    }

    /// A coarse status snapshot for the dashboard's agent list.
    /// "Healthy" = the last reflex step stayed within the fly-derived latency budget.
    pub fn status(&self) -> AgentStatus {
        AgentStatus {
            agent_id: self.agent_id.clone(),
            healthy: (self.last_latency_us as u128) <= REFLEX_BUDGET_US,
            last_latency_us: self.last_latency_us,
            last_error: self.last_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_step_and_reports_latency() {
        let mut sim = AgentSim::new("nzi-001", 1.0 / 500.0);
        let a = sim.tick();
        let b = sim.tick();
        assert_eq!(a.step, 1);
        assert_eq!(b.step, 2);
        // Latency should be well under the 13 ms budget (it's a handful of float ops).
        assert!((b.latency_us as u128) < REFLEX_BUDGET_US);
    }

    #[test]
    fn sim_converges_toward_setpoint() {
        let mut sim = AgentSim::new("nzi-001", 1.0 / 500.0);
        for _ in 0..5000 {
            sim.tick();
        }
        // After enough steps the measured rate should track the setpoint closely.
        assert!(sim.last_error < 0.05, "did not converge: err={}", sim.last_error);
        assert!(sim.status().healthy);
    }

    #[test]
    fn changing_setpoint_is_reflected() {
        let mut sim = AgentSim::new("nzi-001", 1.0 / 500.0);
        sim.set_setpoint(2.0);
        let s = sim.tick();
        assert_eq!(s.setpoint, 2.0);
    }
}
