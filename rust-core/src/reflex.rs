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
}
