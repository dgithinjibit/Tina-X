//! Reflex loop: a delayed-PD attitude stabilizer (the fly's "spine").
//!
//! This is intentionally minimal for Phase 0. It models one rotational axis: given a
//! setpoint (from the slow symbolic brain) and a measured rate (from a rate gyro, standing
//! in for the fly's halteres), it produces a corrective actuator command.
//!
//! The `delay` term reflects the biological reality that the insect sensorimotor system is
//! a PD controller *with delay* (`fly-biomimicry/`). We keep the loop allocation-free and
//! branch-light so a real embedded/WASM target can meet `REFLEX_BUDGET_US`.

/// Filtered-PD-with-delay gains for one axis. (This is a stabilizer, not a tracker.)
#[derive(Clone, Copy, Debug)]
pub struct PdGains {
    pub kp: f32,
    pub kd: f32,
    /// Sensorimotor delay in control steps (fly PD-with-delay). 0 = no modeled delay.
    pub delay_steps: usize,
    /// Derivative low-pass smoothing in [0, 1). 0 = raw derivative (jumpy at high rate);
    /// higher = smoother/"more damped", matching the fly's near-critically-damped haltere
    /// response. An ideal differentiator at 500 Hz amplifies step-to-step noise into
    /// instability, so we always filter it.
    pub d_smoothing: f32,
}

impl Default for PdGains {
    fn default() -> Self {
        // Conservative, stable-by-default starting point for a first-order rate plant at
        // 500 Hz (tau ~20 ms). Derivative-on-measurement + a smoothed D term keep it stable;
        // proper axis-specific tuning against the real Unity dynamics happens in Phase 1 (P1.2).
        Self { kp: 0.9, kd: 0.02, delay_steps: 1, d_smoothing: 0.9 }
    }
}

/// Single-axis delayed-PD stabilizer. Fixed-size delay ring buffer -> no heap in the loop.
pub struct ReflexStabilizer {
    gains: PdGains,
    /// Previous *measured* value, for derivative-on-measurement (avoids derivative kick).
    prev_measured: f32,
    /// Low-pass-filtered derivative state (near-critically-damped haltere response).
    filtered_deriv: f32,
    /// Ring buffer of past measurements, to model sensorimotor delay without allocation.
    delay_ring: [f32; MAX_DELAY],
    ring_head: usize,
}

/// Max modeled sensorimotor delay (steps). Sized generously; embedded targets can shrink it.
pub const MAX_DELAY: usize = 8;

impl ReflexStabilizer {
    pub fn new(gains: PdGains) -> Self {
        assert!(gains.delay_steps < MAX_DELAY, "delay_steps must be < MAX_DELAY");
        Self {
            gains,
            prev_measured: 0.0,
            filtered_deriv: 0.0,
            delay_ring: [0.0; MAX_DELAY],
            ring_head: 0,
        }
    }

    /// One reflex step. `setpoint` and `measured` are the attitude-rate command and reading.
    /// `dt` is the timestep in seconds. Returns the actuator correction command.
    ///
    /// Keep this hot path allocation-free and side-effect-free — it runs at
    /// [`crate::REFLEX_TARGET_HZ`] under [`crate::REFLEX_BUDGET_US`].
    pub fn step(&mut self, setpoint: f32, measured: f32, dt: f32) -> f32 {
        // Model sensorimotor delay on the SENSED signal: the fly reacts to a slightly stale
        // measurement, so we delay `measured` (not the error) through the ring buffer.
        self.delay_ring[self.ring_head] = measured;
        let delayed_idx =
            (self.ring_head + MAX_DELAY - self.gains.delay_steps) % MAX_DELAY;
        let delayed_measured = self.delay_ring[delayed_idx];
        self.ring_head = (self.ring_head + 1) % MAX_DELAY;

        let delayed_error = setpoint - delayed_measured;

        // Derivative-on-MEASUREMENT, not on error. Differentiating the error causes a large
        // "derivative kick" on setpoint changes that destabilizes the loop; differentiating
        // the measured rate avoids it and is biologically faithful (the fly senses its own
        // rotation rate, not an abstract error).
        let raw_derivative = if dt > 0.0 {
            -(delayed_measured - self.prev_measured) / dt
        } else {
            0.0
        };
        self.prev_measured = delayed_measured;

        // Exponential low-pass on the derivative (near-critically-damped haltere response).
        let a = self.gains.d_smoothing.clamp(0.0, 0.999);
        self.filtered_deriv = a * self.filtered_deriv + (1.0 - a) * raw_derivative;

        self.gains.kp * delayed_error + self.gains.kd * self.filtered_deriv
    }
}

/// A minimal 3-component vector for body-axis attitude rates and commands.
///
/// We keep our own tiny type (rather than pulling in a linear-algebra crate) so the reflex
/// path stays dependency-free and trivially `Copy`. Fields are the fly/aircraft body axes:
/// `roll` (x), `pitch` (y), `yaw` (z). All values are angular *rates* (rad/s) for the
/// stabilizer, matching the haltere analogue (the fly senses rotation rate, not angle).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Vec3 {
    /// The zero vector — a convenient "no rotation / no command" value.
    pub const ZERO: Vec3 = Vec3 { roll: 0.0, pitch: 0.0, yaw: 0.0 };

    pub fn new(roll: f32, pitch: f32, yaw: f32) -> Self {
        Self { roll, pitch, yaw }
    }

    /// Largest absolute component — handy for a single "how far off are we?" scalar.
    pub fn max_abs(&self) -> f32 {
        self.roll.abs().max(self.pitch.abs()).max(self.yaw.abs())
    }

    /// True if every component is finite (no NaN/inf) — a cheap health check.
    pub fn is_finite(&self) -> bool {
        self.roll.is_finite() && self.pitch.is_finite() && self.yaw.is_finite()
    }
}

/// Three-axis attitude stabilizer: the fly's full "spine".
///
/// This is just three independent [`ReflexStabilizer`]s — one per body axis — so all the proven
/// delayed-PD / derivative-on-measurement / D-filtering logic is reused, not re-derived. Real
/// flight axes are lightly coupled, but treating them independently is the standard, robust
/// starting point (each axis's gyro feeds its own PD loop). Cross-axis coupling, if we need it,
/// is a Phase-1 tuning concern against the real Unity plant — not a reason to complicate the
/// hot path now.
pub struct AttitudeStabilizer {
    roll: ReflexStabilizer,
    pitch: ReflexStabilizer,
    yaw: ReflexStabilizer,
}

impl AttitudeStabilizer {
    /// Build a 3-axis stabilizer. Pass per-axis gains so each axis can be tuned independently
    /// (a fly's yaw authority differs from its roll authority); use [`Self::uniform`] if you
    /// just want the same gains everywhere.
    pub fn new(roll: PdGains, pitch: PdGains, yaw: PdGains) -> Self {
        Self {
            roll: ReflexStabilizer::new(roll),
            pitch: ReflexStabilizer::new(pitch),
            yaw: ReflexStabilizer::new(yaw),
        }
    }

    /// Convenience: the same gains on all three axes.
    pub fn uniform(gains: PdGains) -> Self {
        Self::new(gains, gains, gains)
    }

    /// One reflex step across all three axes. `setpoint`/`measured` are per-axis rate vectors;
    /// returns the per-axis actuator command vector. Same hot-path rules as
    /// [`ReflexStabilizer::step`] — allocation-free, runs under [`crate::REFLEX_BUDGET_US`].
    pub fn step(&mut self, setpoint: Vec3, measured: Vec3, dt: f32) -> Vec3 {
        Vec3 {
            roll: self.roll.step(setpoint.roll, measured.roll, dt),
            pitch: self.pitch.step(setpoint.pitch, measured.pitch, dt),
            yaw: self.yaw.step(setpoint.yaw, measured.yaw, dt),
        }
    }
}

impl Default for AttitudeStabilizer {
    fn default() -> Self {
        Self::uniform(PdGains::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_error_gives_zero_command() {
        let mut s = ReflexStabilizer::new(PdGains { delay_steps: 0, ..Default::default() });
        assert_eq!(s.step(0.0, 0.0, 0.002), 0.0);
    }

    #[test]
    fn converges_toward_setpoint() {
        // Rate-integrator plant: the command IS angular acceleration and the measured
        // signal IS angular rate, so measured += cmd * dt. This is the physically correct
        // model for a haltere-style RATE stabilizer (the fly regulates its rotation rate),
        // and filtered derivative-on-measurement makes plain PD converge on it without a
        // steady-state offset. Real dynamics arrive in Phase 1 with the Unity plant.
        let mut s = ReflexStabilizer::new(PdGains::default());
        let setpoint = 1.0_f32;
        let mut measured = 0.0_f32;
        let dt = 0.002; // 500 Hz
        for _ in 0..2000 {
            let cmd = s.step(setpoint, measured, dt);
            measured += cmd * dt;
        }
        assert!((setpoint - measured).abs() < 0.05, "did not converge: {measured}");
    }

    #[test]
    fn stays_bounded_no_nan() {
        // Regression guard: derivative-on-measurement + D-filtering must not diverge.
        // (Earlier derivative-on-error caused a derivative kick that blew up to NaN.)
        let mut s = ReflexStabilizer::new(PdGains::default());
        let mut measured = 0.0_f32;
        let dt = 0.002;
        for _ in 0..5000 {
            let cmd = s.step(1.0, measured, dt);
            measured += cmd * dt;
            assert!(measured.is_finite(), "diverged to non-finite: {measured}");
            assert!(measured.abs() < 10.0, "unbounded overshoot: {measured}");
        }
    }

    // --- 3-axis (AttitudeStabilizer) tests -------------------------------------------------

    #[test]
    fn vec3_helpers_behave() {
        let v = Vec3::new(-0.3, 0.7, 0.1);
        assert_eq!(v.max_abs(), 0.7);
        assert!(v.is_finite());
        assert!(!Vec3::new(f32::NAN, 0.0, 0.0).is_finite());
        assert_eq!(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn zero_error_gives_zero_command_all_axes() {
        // With no delay and zero setpoint/measurement, every axis must command exactly zero.
        let g = PdGains { delay_steps: 0, ..Default::default() };
        let mut s = AttitudeStabilizer::uniform(g);
        assert_eq!(s.step(Vec3::ZERO, Vec3::ZERO, 0.002), Vec3::ZERO);
    }

    #[test]
    fn each_axis_converges_to_its_own_setpoint() {
        // Give the three axes DIFFERENT setpoints and confirm each tracks independently —
        // this proves the per-axis loops don't bleed into one another. Rate-integrator plant
        // per axis (measured += cmd*dt), same physically-correct model as the single-axis test.
        let mut s = AttitudeStabilizer::default();
        let setpoint = Vec3::new(1.0, -0.5, 0.25);
        let mut measured = Vec3::ZERO;
        let dt = 0.002; // 500 Hz
        for _ in 0..2000 {
            let cmd = s.step(setpoint, measured, dt);
            measured.roll += cmd.roll * dt;
            measured.pitch += cmd.pitch * dt;
            measured.yaw += cmd.yaw * dt;
        }
        assert!((setpoint.roll - measured.roll).abs() < 0.05, "roll: {measured:?}");
        assert!((setpoint.pitch - measured.pitch).abs() < 0.05, "pitch: {measured:?}");
        assert!((setpoint.yaw - measured.yaw).abs() < 0.05, "yaw: {measured:?}");
    }

    #[test]
    fn three_axis_stays_bounded_no_nan() {
        // Same NaN/divergence regression guard as the single axis, but across all three.
        let mut s = AttitudeStabilizer::default();
        let setpoint = Vec3::new(1.0, 1.0, 1.0);
        let mut measured = Vec3::ZERO;
        let dt = 0.002;
        for _ in 0..5000 {
            let cmd = s.step(setpoint, measured, dt);
            measured.roll += cmd.roll * dt;
            measured.pitch += cmd.pitch * dt;
            measured.yaw += cmd.yaw * dt;
            assert!(measured.is_finite(), "diverged to non-finite: {measured:?}");
            assert!(measured.max_abs() < 10.0, "unbounded overshoot: {measured:?}");
        }
    }
}
