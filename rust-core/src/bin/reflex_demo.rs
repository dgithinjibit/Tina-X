//! reflex-demo — run the delayed-PD reflex loop and measure its per-step latency
//! against the fly-derived budget (`REFLEX_BUDGET_US` = 13 ms).
//!
//! Phase 0 (P0) principle: we MEASURE from day one. This is the smallest possible
//! version of the "benchmark before you lock in real-time" discipline in ROADMAP.md.
//!
//! Run: `cargo run -p nzi-core --bin reflex-demo`

use nzi_core::reflex::{PdGains, ReflexStabilizer};
use nzi_core::{REFLEX_BUDGET_US, REFLEX_TARGET_HZ};
use std::time::Instant;

fn main() {
    let steps: u32 = 10_000;
    let dt = 1.0 / REFLEX_TARGET_HZ as f32;

    let mut stab = ReflexStabilizer::new(PdGains::default());

    // Simulated rate-integrator plant (matches the lib test): command = angular acceleration,
    // measured = angular rate, so measured += cmd * dt. This is the correct model for a
    // haltere-style rate stabilizer. Real dynamics arrive in Phase 1 with Unity.
    let setpoint = 1.0_f32;
    let mut measured = 0.0_f32;

    let mut max_us: u128 = 0;
    let mut sum_us: u128 = 0;
    let mut over_budget: u32 = 0;

    for _ in 0..steps {
        let t0 = Instant::now();
        let cmd = stab.step(setpoint, measured, dt);
        let elapsed = t0.elapsed().as_micros();

        // integrate the toy plant OUTSIDE the timed region
        measured += cmd * dt;

        sum_us += elapsed;
        if elapsed > max_us {
            max_us = elapsed;
        }
        if elapsed > REFLEX_BUDGET_US {
            over_budget += 1;
        }
    }

    let avg_us = sum_us as f64 / steps as f64;
    let final_error = (setpoint - measured).abs();

    println!("Project Nzi — reflex loop micro-benchmark (P0)");
    println!("  target frequency : {REFLEX_TARGET_HZ} Hz (dt = {dt:.6} s)");
    println!("  latency budget   : {REFLEX_BUDGET_US} us (fly ~13 ms stabilization)");
    println!("  steps            : {steps}");
    println!("  avg step latency : {avg_us:.3} us");
    println!("  max step latency : {max_us} us");
    println!("  steps over budget: {over_budget}");
    println!("  final |error|    : {final_error:.4} (converged if < 0.05)");

    if over_budget == 0 && final_error < 0.05 {
        println!("  RESULT: OK — reflex loop is within the fly budget and converges.");
    } else {
        println!("  RESULT: REVIEW — budget exceeded or did not converge (see above).");
    }
}
