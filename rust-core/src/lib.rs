//! Project Nzi — agent core (Phase 0 scaffold).
//!
//! This crate holds the **fast side** of the two-rate brain (see ROADMAP.md / README.md):
//! a low-latency reflex loop inspired by the fly's haltere system. Symbolic reasoning
//! (MeTTa/Hyperon) lives in the *slow* supervisory layer and MUST NOT run inside the
//! reflex path — that is the architectural rule that lets us beat the ">50 Hz control
//! vs. slow model inference" bottleneck documented in `limitations-edge-cases/`.
//!
//! Fly spec we design against (see `fly-biomimicry/`):
//! - flies correct induced rotations in ~5 ms and stabilize in ~13 ms
//! - the sensorimotor system is well modeled by a **PD controller with delay**
//!
//! So the reflex loop here is a delayed-PD stabilizer with a hard latency budget.

pub mod brain;
pub mod reflex;
pub mod roofline;
pub mod telemetry;
pub mod unity_bridge;

/// Hard latency budget for a single reflex step, in microseconds.
///
/// Derived from the fly's ~13 ms stabilization response (`fly-biomimicry/`). If a reflex
/// step exceeds this, we are outside biological plausibility and the control loop is at risk.
pub const REFLEX_BUDGET_US: u128 = 13_000;

/// Target reflex control frequency (Hz). Robot control loops typically run >50 Hz for
/// stability (`limitations-edge-cases/` bottleneck #3). We aim well above that.
pub const REFLEX_TARGET_HZ: u32 = 500;
