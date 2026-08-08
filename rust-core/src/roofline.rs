//! Roofline analysis + profiling — the transferable ideas from the "How To Scale Your Model"
//! book (jax-ml/scaling-book), adapted to Nzi's edge/symbolic regime.
//!
//! # For a junior dev: what is a "roofline"?
//!
//! Every operation moves some BYTES and does some FLOPs (math operations). Two things can
//! limit how fast it runs:
//!   * COMPUTE-bound  — you're waiting on the processor to do arithmetic.
//!   * MEMORY-bound   — you're waiting on data to move to/from memory (or over a network).
//!
//! The **roofline model** decides which one you're hitting using a single ratio:
//!
//!   arithmetic_intensity = FLOPs / bytes_moved      (units: FLOP per byte)
//!
//! and a hardware "ridge point":
//!
//!   ridge_point = peak_compute (FLOP/s) / peak_bandwidth (bytes/s)   (units: FLOP per byte)
//!
//! Rule:
//!   * intensity <  ridge  -> MEMORY-bound  (add compute won't help; move less data)
//!   * intensity >= ridge  -> COMPUTE-bound (faster memory won't help; do less math)
//!
//! # Why this matters for Nzi (not just for TPUs)
//!
//! The book targets LLMs on TPUs, but the *lens* is universal. Our own Phase-0 findings were
//! secretly roofline results:
//!   * the O(n) MeTTa `space.query()` (docs/benchmarks) is MEMORY/scan-bound — it moves the
//!     whole space; more CPU wouldn't help (that's why we partition it — ADR 0002).
//!   * the reflex loop is tiny-compute + tiny-memory → dominated by fixed per-step overhead.
//!
//! This module gives us a tiny, dependency-free way to make that reasoning explicit and testable.

use serde::{Deserialize, Serialize};

/// A hardware roofline: the two peak numbers that define the ridge point.
///
/// These are ORDER-OF-MAGNITUDE figures you plug in for the target device (a laptop CPU, a
/// microcontroller, etc.). You don't need them exact — roofline is for reasoning, not billing.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Roofline {
    /// Peak compute in FLOP/s (floating-point ops per second).
    pub peak_flops_per_s: f64,
    /// Peak memory bandwidth in bytes/s.
    pub peak_bytes_per_s: f64,
}

impl Roofline {
    /// The ridge point in FLOP/byte: below it you're memory-bound, at/above it compute-bound.
    pub fn ridge_point(&self) -> f64 {
        // Guard against a nonsensical zero-bandwidth config.
        if self.peak_bytes_per_s <= 0.0 {
            return f64::INFINITY;
        }
        self.peak_flops_per_s / self.peak_bytes_per_s
    }

    /// A rough desktop/laptop CPU default, useful for local reasoning and tests.
    /// (~50 GFLOP/s single-thread-ish, ~20 GB/s memory bandwidth → ridge ~2.5 FLOP/byte.)
    pub fn laptop_cpu() -> Self {
        Self { peak_flops_per_s: 50e9, peak_bytes_per_s: 20e9 }
    }
}

/// Whether an operation is limited by compute or by data movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bound {
    Compute,
    Memory,
}

/// A measured (or estimated) operation: how much math and how much data movement it involves.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OpProfile {
    /// Total floating-point operations performed.
    pub flops: f64,
    /// Total bytes moved to/from memory (or over the network).
    pub bytes_moved: f64,
}

impl OpProfile {
    pub fn new(flops: f64, bytes_moved: f64) -> Self {
        Self { flops, bytes_moved }
    }

    /// Arithmetic intensity = FLOPs per byte moved. This is the x-axis of the roofline plot.
    pub fn arithmetic_intensity(&self) -> f64 {
        if self.bytes_moved <= 0.0 {
            // No data movement -> effectively "infinitely compute-heavy per byte".
            return f64::INFINITY;
        }
        self.flops / self.bytes_moved
    }

    /// Classify this op against a given hardware roofline.
    pub fn classify(&self, hw: &Roofline) -> Bound {
        if self.arithmetic_intensity() < hw.ridge_point() {
            Bound::Memory
        } else {
            Bound::Compute
        }
    }

    /// The roofline lower-bound on runtime (seconds): the best you could POSSIBLY do on this
    /// hardware. Real runtime is always >= this; comparing the two tells you how much headroom
    /// (or overhead) you have. This is the book's "am I near the roof?" check.
    pub fn min_runtime_s(&self, hw: &Roofline) -> f64 {
        let compute_time = if hw.peak_flops_per_s > 0.0 {
            self.flops / hw.peak_flops_per_s
        } else {
            f64::INFINITY
        };
        let memory_time = if hw.peak_bytes_per_s > 0.0 {
            self.bytes_moved / hw.peak_bytes_per_s
        } else {
            f64::INFINITY
        };
        // You are bottlenecked by whichever takes LONGER (they overlap in the ideal case).
        compute_time.max(memory_time)
    }
}

// ---------------------------------------------------------------------------------------------
// A tiny profiling helper: time a closure and report against the roofline.
// ---------------------------------------------------------------------------------------------

/// The result of profiling one operation: measured time vs. the roofline ideal.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProfileResult {
    /// Measured wall-clock seconds for the op.
    pub measured_s: f64,
    /// Roofline lower bound (seconds) for the op on this hardware.
    pub min_runtime_s: f64,
    /// Which resource the op is theoretically bound by.
    pub bound: Bound,
    /// Arithmetic intensity (FLOP/byte).
    pub arithmetic_intensity: f64,
}

impl ProfileResult {
    /// "Efficiency" = how close the measured time is to the theoretical best (0..1].
    /// Near 1.0 = you're basically at the roof; near 0.0 = huge fixed/overhead cost dominates.
    pub fn efficiency(&self) -> f64 {
        if self.measured_s <= 0.0 {
            return 0.0;
        }
        (self.min_runtime_s / self.measured_s).min(1.0)
    }
}

/// Run `op` `iters` times, measure the average, and classify it against `hw`.
///
/// `profile.flops` / `profile.bytes_moved` describe ONE call of `op`. Keep `op` side-effect-light
/// so the measurement reflects the work, not allocation noise.
pub fn profile<F: FnMut()>(
    hw: &Roofline,
    profile: OpProfile,
    iters: u32,
    mut op: F,
) -> ProfileResult {
    use std::time::Instant;

    // Warm up once so we don't measure first-call effects (caches, branch prediction).
    op();

    let start = Instant::now();
    for _ in 0..iters {
        op();
    }
    let total_s = start.elapsed().as_secs_f64();
    let measured_s = if iters > 0 { total_s / iters as f64 } else { total_s };

    ProfileResult {
        measured_s,
        min_runtime_s: profile.min_runtime_s(hw),
        bound: profile.classify(hw),
        arithmetic_intensity: profile.arithmetic_intensity(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ridge_point_is_flops_over_bandwidth() {
        let hw = Roofline { peak_flops_per_s: 100.0, peak_bytes_per_s: 25.0 };
        assert_eq!(hw.ridge_point(), 4.0);
    }

    #[test]
    fn low_intensity_op_is_memory_bound() {
        // Move lots of bytes, do little math -> memory-bound.
        let hw = Roofline::laptop_cpu(); // ridge ~2.5 FLOP/byte
        let op = OpProfile::new(/*flops*/ 10.0, /*bytes*/ 1_000.0); // intensity 0.01
        assert_eq!(op.classify(&hw), Bound::Memory);
    }

    #[test]
    fn high_intensity_op_is_compute_bound() {
        let hw = Roofline::laptop_cpu();
        let op = OpProfile::new(/*flops*/ 1_000_000.0, /*bytes*/ 8.0); // huge intensity
        assert_eq!(op.classify(&hw), Bound::Compute);
    }

    #[test]
    fn zero_bytes_is_compute_bound_not_a_panic() {
        let hw = Roofline::laptop_cpu();
        let op = OpProfile::new(100.0, 0.0);
        assert!(op.arithmetic_intensity().is_infinite());
        assert_eq!(op.classify(&hw), Bound::Compute);
    }

    #[test]
    fn min_runtime_takes_the_slower_resource() {
        let hw = Roofline { peak_flops_per_s: 100.0, peak_bytes_per_s: 100.0 };
        // compute_time = 200/100 = 2s; memory_time = 50/100 = 0.5s -> bound by compute (2s).
        let op = OpProfile::new(200.0, 50.0);
        assert!((op.min_runtime_s(&hw) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn efficiency_is_bounded_0_to_1() {
        let r = ProfileResult {
            measured_s: 10.0,
            min_runtime_s: 2.0,
            bound: Bound::Memory,
            arithmetic_intensity: 0.1,
        };
        assert!((r.efficiency() - 0.2).abs() < 1e-9);

        // Measured faster than the "ideal" (noise) should still clamp to 1.0, never above.
        let r2 = ProfileResult { measured_s: 1.0, min_runtime_s: 5.0, ..r };
        assert_eq!(r2.efficiency(), 1.0);
    }

    #[test]
    fn profile_runs_and_classifies() {
        let hw = Roofline::laptop_cpu();
        // A trivially memory-ish op: sum a small array (low arithmetic intensity).
        let data = [1.0_f64; 64];
        let op_profile = OpProfile::new(/*flops*/ 64.0, /*bytes*/ 64.0 * 8.0);
        let mut sink = 0.0_f64;
        let res = profile(&hw, op_profile, 100, || {
            sink += data.iter().sum::<f64>();
        });
        assert!(res.measured_s >= 0.0);
        assert_eq!(res.bound, Bound::Memory);
        assert!(sink > 0.0); // ensure the op wasn't optimized away
    }
}
