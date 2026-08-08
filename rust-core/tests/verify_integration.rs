//! Fault-injection integration tests for the action-gate (P2.3).
//!
//! # For a junior dev
//! `src/verify.rs`'s unit tests use a FakeBrain and check the gate's transport/parsing in
//! isolation. THIS file is the real thing: it loads the actual `metta-logic/verification/*.metta`
//! rules and drives MeTTa through the subprocess brain, then FEEDS THE GATE BAD ACTIONS on purpose
//! and asserts each of the five agent-hallucination types is *caught* (rejected), not acted on.
//! It also confirms a well-formed, safe action is approved.
//!
//! This is the end-to-end proof of the verification moat: injected faults never reach the reflex
//! layer. It mirrors `metta-logic/run_verify_smoke.py`, but exercises the RUST gate that
//! production actually uses.
//!
//! Like `brain_integration.rs`, it **skips itself gracefully** when the venv isn't set up, so the
//! suite stays green everywhere while proving the real gate wherever MeTTa is installed.
//!
//! Run it with output visible:
//!   cargo test -p nzi-core --test verify_integration -- --nocapture

use nzi_core::brain::SubprocessBrain;
use nzi_core::verify::{Context, Gate, Setpoint, Verdict};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust-core should have a parent directory")
        .to_path_buf()
}

/// Build the real gate, or return None (skipping the test) if MeTTa isn't available.
fn gate_or_skip(test_name: &str) -> Option<Gate<SubprocessBrain>> {
    let root = project_root();
    // Reuse the brain's availability probe so we skip for the same reason brain_integration does.
    if !SubprocessBrain::from_project_root(root.clone()).is_available() {
        eprintln!(
            "SKIP {test_name}: MeTTa venv not found. Run the setup in docs/DEV_SETUP.md \
             (python3 -m venv .venv && .venv/bin/pip install hyperon) to exercise this test."
        );
        return None;
    }
    match Gate::from_project_root(&root) {
        Ok(g) => Some(g),
        Err(e) => panic!("venv present but gate failed to build: {e}"),
    }
}

/// A context that passes every check; each fault case tweaks exactly one field.
fn good_ctx() -> Context {
    Context {
        measured: Setpoint::new(0.4, 0.0, 0.1),
        fresh: true,
        finite: true,
        source: "local-brain".to_string(),
        mode: "free".to_string(),
    }
}

const GOOD_ACTION: Setpoint = Setpoint { roll: 0.5, pitch: 0.0, yaw: 0.2 };

#[test]
fn safe_action_is_approved() {
    let Some(gate) = gate_or_skip("safe_action_is_approved") else { return };
    let verdict = gate.check(GOOD_ACTION, &good_ctx()).expect("gate call should succeed");
    assert_eq!(verdict, Verdict::Approved, "a safe, well-attributed action must be approved");
}

#[test]
fn perception_fault_is_caught() {
    // Stale telemetry: acting on it is a perception hallucination.
    let Some(gate) = gate_or_skip("perception_fault_is_caught") else { return };
    let ctx = Context { fresh: false, ..good_ctx() };
    let verdict = gate.check(GOOD_ACTION, &ctx).expect("gate call should succeed");
    assert_eq!(verdict, Verdict::Rejected("perception".to_string()));
}

#[test]
fn communication_fault_is_caught() {
    // Unknown, untrusted source: an unattributed command.
    let Some(gate) = gate_or_skip("communication_fault_is_caught") else { return };
    let ctx = Context { source: "unknown".to_string(), ..good_ctx() };
    let verdict = gate.check(GOOD_ACTION, &ctx).expect("gate call should succeed");
    assert_eq!(verdict, Verdict::Rejected("communication".to_string()));
}

#[test]
fn reasoning_fault_is_caught() {
    // 9.0 rad/s exceeds the physical max-rate envelope: an internally-impossible plan.
    let Some(gate) = gate_or_skip("reasoning_fault_is_caught") else { return };
    let action = Setpoint::new(9.0, 0.0, 0.2);
    let verdict = gate.check(action, &good_ctx()).expect("gate call should succeed");
    assert_eq!(verdict, Verdict::Rejected("reasoning".to_string()));
}

#[test]
fn execution_fault_is_caught() {
    // 0.4 -> 2.5 is a 2.1 step (past actuator slew authority) though 2.5 < max-rate, so it slips
    // past reasoning and must be caught by execution.
    let Some(gate) = gate_or_skip("execution_fault_is_caught") else { return };
    let action = Setpoint::new(2.5, 0.0, 0.2);
    let verdict = gate.check(action, &good_ctx()).expect("gate call should succeed");
    assert_eq!(verdict, Verdict::Rejected("execution".to_string()));
}

#[test]
fn memorization_fault_is_caught() {
    // hold-level mode forbids non-zero roll/pitch; a 0.5 roll setpoint forgets that invariant.
    let Some(gate) = gate_or_skip("memorization_fault_is_caught") else { return };
    let ctx = Context { mode: "hold-level".to_string(), ..good_ctx() };
    let verdict = gate.check(GOOD_ACTION, &ctx).expect("gate call should succeed");
    assert_eq!(verdict, Verdict::Rejected("memorization".to_string()));
}

#[test]
fn all_five_hallucination_types_are_caught() {
    // A single sweep proving the moat is complete: none of the five bad actions is approved.
    let Some(gate) = gate_or_skip("all_five_hallucination_types_are_caught") else { return };

    let faults: [(&str, Setpoint, Context); 5] = [
        ("perception", GOOD_ACTION, Context { fresh: false, ..good_ctx() }),
        ("communication", GOOD_ACTION, Context { source: "unknown".to_string(), ..good_ctx() }),
        ("reasoning", Setpoint::new(9.0, 0.0, 0.2), good_ctx()),
        ("execution", Setpoint::new(2.5, 0.0, 0.2), good_ctx()),
        ("memorization", GOOD_ACTION, Context { mode: "hold-level".to_string(), ..good_ctx() }),
    ];

    for (name, action, ctx) in &faults {
        let verdict = gate.check(*action, ctx).expect("gate call should succeed");
        assert!(!verdict.is_approved(), "{name} fault was wrongly approved: {verdict:?}");
        assert_eq!(verdict, Verdict::Rejected(name.to_string()), "{name} caught by wrong check");
    }
}
