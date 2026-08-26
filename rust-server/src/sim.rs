//! A tiny live simulation that drives the reflex loop and produces telemetry.
//!
//! # For a junior dev
//! The dashboard needs *something* to show. Until we have real hardware (Phase 6) or the Unity
//! sim (Phase 1), this module runs the SAME reflex stabilizer from `tina-core` against a toy
//! plant and emits [`ReflexSample`]s — so judges see a real, running control loop, not a mockup.
//!
//! This is pure logic (no async, no HTTP) so it is easy to unit-test. The server layer just
//! calls [`AgentSim::tick`] on a timer and forwards the samples.

use tina_core::reflex::{AttitudeStabilizer, Vec3};
use tina_core::telemetry::{AgentStatus, Axis3, AttitudeSample, ReflexSample};
use tina_core::REFLEX_BUDGET_US;
use std::time::Instant;

/// Convert a control-layer `Vec3` into the telemetry-layer `Axis3` (wire type).
fn to_axis3(v: Vec3) -> Axis3 {
    Axis3 { roll: v.roll, pitch: v.pitch, yaw: v.yaw }
}

/// One simulated TINA-X agent: a 3-axis attitude stabilizer + a toy per-axis rate-integrator plant.
pub struct AgentSim {
    agent_id: String,
    stab: AttitudeStabilizer,
    /// Per-axis target body rates the "slow brain" wants held.
    setpoint: Vec3,
    /// Per-axis measured body rates (evolves via the toy plant).
    measured: Vec3,
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
            stab: AttitudeStabilizer::default(),
            // Distinct per-axis targets so the dashboard shows three genuinely independent
            // traces (roll climbs to +1, pitch to -0.5, yaw to +0.25) rather than three copies.
            setpoint: Vec3::new(1.0, -0.5, 0.25),
            measured: Vec3::ZERO,
            dt,
            step: 0,
            last_latency_us: 0,
            last_error: 0.0,
        }
    }

    /// Change the target body rates the agent is trying to hold (the "slow brain" would set this).
    pub fn set_setpoint(&mut self, setpoint: Vec3) {
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

        // Integrate the toy per-axis rate-integrator plant OUTSIDE the timed region: for a rate
        // stabilizer the command IS angular acceleration, so measured_rate += cmd * dt per axis.
        self.measured.roll += command.roll * self.dt;
        self.measured.pitch += command.pitch * self.dt;
        self.measured.yaw += command.yaw * self.dt;

        self.step += 1;
        self.last_latency_us = latency_us;
        // Worst-axis error — the single "how far off is the agent?" scalar the status uses.
        self.last_error = Vec3::new(
            self.setpoint.roll - self.measured.roll,
            self.setpoint.pitch - self.measured.pitch,
            self.setpoint.yaw - self.measured.yaw,
        )
        .max_abs();

        ReflexSample {
            step: self.step,
            // Primary axis (roll) fills the legacy scalar fields for backward compatibility.
            setpoint: self.setpoint.roll,
            measured: self.measured.roll,
            command: command.roll,
            latency_us,
            attitude: Some(AttitudeSample {
                setpoint: to_axis3(self.setpoint),
                measured: to_axis3(self.measured),
                command: to_axis3(command),
            }),
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
        let mut sim = AgentSim::new("tina-001", 1.0 / 500.0);
        let a = sim.tick();
        let b = sim.tick();
        assert_eq!(a.step, 1);
        assert_eq!(b.step, 2);
        // Latency should be well under the 13 ms budget (it's a handful of float ops).
        assert!((b.latency_us as u128) < REFLEX_BUDGET_US);
    }

    #[test]
    fn tick_emits_three_axis_attitude() {
        // The richer telemetry must be present, and its roll axis must equal the legacy scalars.
        let mut sim = AgentSim::new("tina-001", 1.0 / 500.0);
        let s = sim.tick();
        let att = s.attitude.expect("attitude should be populated by the multi-axis sim");
        assert_eq!(att.setpoint.roll, s.setpoint);
        assert_eq!(att.measured.roll, s.measured);
        assert_eq!(att.command.roll, s.command);
    }

    #[test]
    fn sim_converges_on_all_axes() {
        let mut sim = AgentSim::new("tina-001", 1.0 / 500.0);
        for _ in 0..5000 {
            sim.tick();
        }
        // last_error is the WORST-axis error, so this asserts every axis converged.
        assert!(sim.last_error < 0.05, "did not converge: worst-axis err={}", sim.last_error);
        assert!(sim.status().healthy);
    }

    #[test]
    fn changing_setpoint_is_reflected() {
        let mut sim = AgentSim::new("tina-001", 1.0 / 500.0);
        sim.set_setpoint(Vec3::new(2.0, 0.0, 0.0));
        let s = sim.tick();
        assert_eq!(s.setpoint, 2.0); // roll axis surfaces in the legacy scalar
        assert_eq!(s.attitude.unwrap().setpoint.roll, 2.0);
    }
}
