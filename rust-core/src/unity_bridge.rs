//! Unity ↔ Rust reflex bridge — the RUST side of the wire contract.
//!
//! These types mirror `unity-sim/BRIDGE_CONTRACT.md` exactly. Unity (the simulated body) sends a
//! [`GyroReading`] every fixed physics tick; we run the reflex stabilizer and send back an
//! [`ActuatorCommand`]. The controller lives only here — Unity applies whatever torque we return.
//!
//! # For a junior dev
//! This module is deliberately transport-agnostic: it defines the MESSAGES and a pure
//! [`respond`] function that turns a reading into a command using a stabilizer. The actual TCP
//! server (Phase 1.2) is a thin wrapper that reads a line, calls [`respond`], writes a line. We
//! keep it split this way so the contract + control response are unit-testable WITHOUT a socket
//! and WITHOUT Unity (which can't run in this environment).

use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::reflex::{AttitudeStabilizer, Vec3};
use crate::telemetry::Axis3;

/// Unity → Rust: one gyro reading for a fixed physics tick. Matches `BRIDGE_CONTRACT.md`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GyroReading {
    /// Unity's FixedUpdate counter (for correlating logs across the wire).
    pub step: u64,
    /// Fixed timestep in seconds (Unity `Time.fixedDeltaTime`).
    pub dt: f32,
    /// Measured body angular rates (rad/s) from the simulated gyro.
    pub measured: Axis3,
}

/// Rust → Unity: the actuator command to apply this tick. Matches `BRIDGE_CONTRACT.md`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActuatorCommand {
    /// Echoes the request `step` so Unity can assert the exchange stayed aligned.
    pub step: u64,
    /// Per-axis actuator command from the stabilizer, nominally in [-1, 1].
    pub command: Axis3,
    /// How long the reflex step took (µs). Must stay under [`crate::REFLEX_BUDGET_US`].
    pub latency_us: u32,
}

/// Convert the wire triple into the control-layer vector.
fn to_vec3(a: Axis3) -> Vec3 {
    Vec3::new(a.roll, a.pitch, a.yaw)
}

/// Convert the control-layer vector back into the wire triple.
fn to_axis3(v: Vec3) -> Axis3 {
    Axis3 { roll: v.roll, pitch: v.pitch, yaw: v.yaw }
}

/// Turn one [`GyroReading`] into one [`ActuatorCommand`] using `stab`, targeting `setpoint`.
///
/// This is THE bridge behaviour, minus any I/O: the socket server just does
/// `respond(&mut stab, setpoint, read_reading()?)` and writes the result. We time only the
/// controller call so `latency_us` reflects the real reflex cost (the number Unity/the dashboard
/// checks against the 13 ms budget).
pub fn respond(stab: &mut AttitudeStabilizer, setpoint: Vec3, reading: GyroReading) -> ActuatorCommand {
    let t0 = Instant::now();
    let command = stab.step(setpoint, to_vec3(reading.measured), reading.dt);
    let latency_us = t0.elapsed().as_micros() as u32;

    ActuatorCommand { step: reading.step, command: to_axis3(command), latency_us }
}

/// Parse one line of the Unity→Rust stream into a [`GyroReading`].
///
/// Wraps `serde_json` so the socket server has a single, well-named parse point. Returns the
/// serde error on malformed input (the server logs it and skips the frame rather than crashing).
pub fn parse_reading(line: &str) -> Result<GyroReading, serde_json::Error> {
    serde_json::from_str(line.trim())
}

/// Serialize an [`ActuatorCommand`] into one newline-terminated line for the wire.
pub fn encode_command(cmd: &ActuatorCommand) -> String {
    // Contract is line-delimited JSON: one object, one trailing '\n'.
    let mut s = serde_json::to_string(cmd).expect("ActuatorCommand always serializes");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflex::PdGains;

    fn axis(roll: f32, pitch: f32, yaw: f32) -> Axis3 {
        Axis3 { roll, pitch, yaw }
    }

    #[test]
    fn gyro_reading_matches_the_documented_json_shape() {
        // Lock the Unity→Rust contract: {step, dt, measured:{roll,pitch,yaw}}.
        let line = r#"{"step":1234,"dt":0.002,"measured":{"roll":0.12,"pitch":-0.03,"yaw":0.0}}"#;
        let r = parse_reading(line).unwrap();
        assert_eq!(r.step, 1234);
        assert_eq!(r.dt, 0.002);
        assert_eq!(r.measured.roll, 0.12);
    }

    #[test]
    fn actuator_command_encodes_as_one_json_line() {
        let cmd = ActuatorCommand { step: 7, command: axis(0.4, -0.1, 0.0), latency_us: 7 };
        let line = encode_command(&cmd);
        assert!(line.ends_with('\n'), "must be newline-terminated");
        assert_eq!(line.matches('\n').count(), 1, "exactly one line");
        assert!(line.contains("\"step\":7"));
        // Round-trips back to the same command.
        let back: ActuatorCommand = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(cmd, back);
    }

    #[test]
    fn respond_echoes_step_and_stays_within_budget() {
        let mut stab = AttitudeStabilizer::default();
        let reading = GyroReading { step: 99, dt: 0.002, measured: axis(0.0, 0.0, 0.0) };
        let cmd = respond(&mut stab, Vec3::ZERO, reading);
        assert_eq!(cmd.step, 99, "must echo the request step for alignment checks");
        assert!((cmd.latency_us as u128) < crate::REFLEX_BUDGET_US);
    }

    #[test]
    fn respond_drives_measured_toward_setpoint_over_many_ticks() {
        // Simulate Unity's physics loop: apply the returned command to a rate-integrator plant
        // and feed the new rate back. The reflex loop should pull the body to the setpoint —
        // proving the bridge wiring (reading -> stabilizer -> command) is correct end to end.
        let mut stab = AttitudeStabilizer::uniform(PdGains::default());
        let setpoint = Vec3::new(1.0, -0.5, 0.25);
        let mut measured = Vec3::ZERO;
        let dt = 0.002;
        for step in 0..2000u64 {
            let reading = GyroReading { step, dt, measured: to_axis3(measured) };
            let cmd = respond(&mut stab, setpoint, reading);
            // Unity's job: integrate the command as angular acceleration.
            measured.roll += cmd.command.roll * dt;
            measured.pitch += cmd.command.pitch * dt;
            measured.yaw += cmd.command.yaw * dt;
        }
        assert!((setpoint.roll - measured.roll).abs() < 0.05, "roll: {measured:?}");
        assert!((setpoint.pitch - measured.pitch).abs() < 0.05, "pitch: {measured:?}");
        assert!((setpoint.yaw - measured.yaw).abs() < 0.05, "yaw: {measured:?}");
    }

    #[test]
    fn malformed_line_is_an_error_not_a_panic() {
        assert!(parse_reading("not json").is_err());
        assert!(parse_reading("{}").is_err()); // missing required fields
    }
}
