//! Project Nzi ZK POC (Cairo) — the "Verifiable Safe-to-Fly Decision" (see docs/zk/).
//!
//! # For a junior dev
//! This is the SAME rule as the Rust POC and the MeTTa brain:
//!
//!     decision = 1 (fly) if wind <= tolerance, else 0 (refuse)
//!
//! In Cairo, writing the rule as a function and executing it on a STARK prover produces a proof
//! that the rule was applied correctly. Here we implement the rule and prove its correctness with
//! unit tests (`scarb cairo-test`). The public/private input split (hide `wind`, reveal
//! `tolerance` + `decision`) is handled at the prover boundary in a later step — this file pins
//! down the trusted logic.

/// The safety rule. Returns 1 (fly) when `wind <= tolerance`, else 0 (refuse).
///
/// We use `u32` (bounded integers) so the comparison is well-defined, mirroring the Rust POC's
/// bounded values. Cairo's native field is huge; bounded ints keep the semantics obvious.
pub fn safe_to_fly(wind: u32, tolerance: u32) -> u32 {
    if wind <= tolerance {
        1
    } else {
        0
    }
}

/// Verify a *claimed* decision against the rule for given inputs.
///
/// This is the function a prover would run: it returns `true` only if `claimed_decision` is the
/// correct rule output. Running THIS on a prover yields a proof that the agent's decision was
/// legitimate — the ZK analogue of the Rust circuit's constraint check.
pub fn decision_is_valid(wind: u32, tolerance: u32, claimed_decision: u32) -> bool {
    safe_to_fly(wind, tolerance) == claimed_decision
}

#[cfg(test)]
mod tests {
    use super::{safe_to_fly, decision_is_valid};

    #[test]
    fn safe_below_tolerance() {
        assert(safe_to_fly(8, 12) == 1, 'wind below tol should fly');
    }

    #[test]
    fn boundary_is_safe() {
        // The rule uses <=, so wind == tolerance is still "fly".
        assert(safe_to_fly(12, 12) == 1, 'boundary should fly');
    }

    #[test]
    fn unsafe_above_tolerance() {
        assert(safe_to_fly(20, 12) == 0, 'wind above tol should refuse');
    }

    #[test]
    fn honest_decisions_validate() {
        assert(decision_is_valid(8, 12, 1), 'honest fly ok');
        assert(decision_is_valid(20, 12, 0), 'honest refuse ok');
    }

    #[test]
    fn lying_decisions_are_rejected() {
        // Claiming "fly" (1) when wind 20 > tol 12 must be rejected by the validator — the
        // Cairo analogue of "a lying prover can't produce a valid proof".
        assert(!decision_is_valid(20, 12, 1), 'lying fly rejected');
        assert(!decision_is_valid(8, 12, 0), 'lying refuse rejected');
    }
}
