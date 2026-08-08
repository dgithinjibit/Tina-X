//! End-to-end integration test for the supervisory loop (P2.5/P2.6).
//!
//! # For a junior dev
//! `src/supervise.rs`'s unit tests use a FakeBrain to check the supervisor's state machine in
//! isolation. THIS file drives the REAL chain: it loads the actual knowledge base
//! (`metta-logic/knowledge/*.metta`) AND the verification rules, runs them through MeTTa via the
//! subprocess brain, and confirms the two-rate hand-off behaves:
//!
//!   * a healthy tick in `free` mode -> the brain proposes an in-envelope setpoint, the gate
//!     approves it, and the reflex target is updated;
//!   * a `hold-level` tick -> the brain proposes zero rates and it's approved (consistent with the
//!     memorization invariant);
//!   * a tick with STALE telemetry -> the gate rejects on perception and the supervisor HOLDS the
//!     last-safe setpoint (governance / fail-closed).
//!
//! Skips gracefully when the venv isn't set up, like the other integration tests.
//!
//! Run:  cargo test -p nzi-core --test supervise_integration -- --nocapture

use nzi_core::brain::SubprocessBrain;
use nzi_core::reflex::Vec3;
use nzi_core::supervise::{Disposition, Supervisor, Telemetry};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust-core should have a parent directory")
        .to_path_buf()
}

fn supervisor_or_skip(test_name: &str) -> Option<Supervisor<SubprocessBrain>> {
    let root = project_root();
    if !SubprocessBrain::from_project_root(root.clone()).is_available() {
        eprintln!(
            "SKIP {test_name}: MeTTa venv not found. Run the setup in docs/DEV_SETUP.md \
             to exercise this test."
        );
        return None;
    }
    match Supervisor::from_project_root(&root) {
        Ok(s) => Some(s),
        Err(e) => panic!("venv present but supervisor failed to build: {e}"),
    }
}

fn tlm(mode: &str, fresh: bool) -> Telemetry {
    Telemetry {
        measured: Vec3::new(0.4, 0.0, 0.1),
        fresh,
        finite: true,
        mode: mode.to_string(),
    }
}

#[test]
fn free_mode_proposal_is_approved_and_forwarded() {
    let Some(mut sup) = supervisor_or_skip("free_mode_proposal_is_approved_and_forwarded") else {
        return;
    };
    let out = sup.tick(&tlm("free", true), 1);
    assert_eq!(out.disposition, Disposition::Approved, "healthy free-mode tick must approve");
    // The KB's free-mode target, clamped into the envelope: (0.5, 0.0, 0.2).
    assert_eq!(out.setpoint, Vec3::new(0.5, 0.0, 0.2));
    assert!(out.decision.verified);
    assert_eq!(sup.last_safe(), Vec3::new(0.5, 0.0, 0.2));
}

#[test]
fn hold_level_proposes_zero_and_is_approved() {
    let Some(mut sup) = supervisor_or_skip("hold_level_proposes_zero_and_is_approved") else {
        return;
    };
    let out = sup.tick(&tlm("hold-level", true), 1);
    assert_eq!(out.disposition, Disposition::Approved);
    assert_eq!(out.setpoint, Vec3::ZERO, "hold-level must command zero rates");
}

#[test]
fn stale_telemetry_is_refused_and_last_safe_is_held() {
    let Some(mut sup) = supervisor_or_skip("stale_telemetry_is_refused_and_last_safe_is_held")
    else {
        return;
    };

    // 1. A healthy tick establishes a non-zero last-safe setpoint.
    let ok = sup.tick(&tlm("free", true), 1);
    assert_eq!(ok.disposition, Disposition::Approved);
    let safe = sup.last_safe();
    assert_eq!(safe, Vec3::new(0.5, 0.0, 0.2));

    // 2. Now the same proposal but with STALE telemetry: the gate's perception check rejects it,
    //    and governance holds the previously-approved setpoint rather than acting on bad sensing.
    let out = sup.tick(&tlm("free", false), 2);
    assert_eq!(out.disposition, Disposition::Refused("perception".to_string()));
    assert_eq!(out.setpoint, safe, "must hold the last-safe setpoint, not act on stale data");
    assert!(!out.decision.verified);
    assert_eq!(sup.last_safe(), safe, "last-safe is unchanged after a refusal");
}
