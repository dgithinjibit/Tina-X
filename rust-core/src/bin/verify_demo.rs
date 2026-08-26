//! verify-demo — show the action-gate catching each hallucination type end-to-end (P2.2/P2.3).
//!
//! The human-runnable companion to the fault-injection tests. It builds the REAL gate (loads the
//! `metta-logic/verification/*.metta` rules and drives MeTTa via the subprocess brain), then runs
//! one good setpoint plus one representative BAD setpoint for each of the five agent-hallucination
//! types, printing the verdict for each. You should see the good one Approved and every bad one
//! Rejected, naming the check that caught it.
//!
//! Run:  cargo run -p tina-core --bin verify-demo
//! (Requires the venv from docs/DEV_SETUP.md; otherwise it prints a friendly setup hint.)

use tina_core::verify::{Context, Gate, Setpoint};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// A context that passes every check; each bad case tweaks exactly one thing to trip one check.
fn good_ctx() -> Context {
    Context {
        measured: Setpoint::new(0.4, 0.0, 0.1),
        fresh: true,
        finite: true,
        source: "local-brain".to_string(),
        mode: "free".to_string(),
    }
}

fn main() {
    println!("Project TINA-X — action-gate demo (Phase 2, verification moat)");

    let root = project_root();
    let gate = match Gate::from_project_root(&root) {
        Ok(g) => g,
        Err(e) => {
            println!("  Could not build the gate: {e}");
            println!("  (Ensure the venv is set up — see docs/DEV_SETUP.md — and the");
            println!("   metta-logic/verification/*.metta rules are present.)");
            return;
        }
    };

    let good = Setpoint::new(0.5, 0.0, 0.2);

    // (label, action, context) — one good, then one bad per hallucination type.
    let cases: Vec<(&str, Setpoint, Context)> = vec![
        ("approved-good", good, good_ctx()),
        // Perception: telemetry not fresh.
        ("perception (stale telemetry)", good, Context { fresh: false, ..good_ctx() }),
        // Communication: unknown, untrusted source.
        (
            "communication (unknown source)",
            good,
            Context { source: "unknown".to_string(), ..good_ctx() },
        ),
        // Reasoning: 9.0 rad/s exceeds the physical max-rate.
        ("reasoning (rate over envelope)", Setpoint::new(9.0, 0.0, 0.2), good_ctx()),
        // Execution: 0.4 -> 2.5 is a 2.1 step, past actuator slew authority.
        ("execution (step too large)", Setpoint::new(2.5, 0.0, 0.2), good_ctx()),
        // Memorization: hold-level mode forbids non-zero roll/pitch.
        (
            "memorization (violates hold-level)",
            good,
            Context { mode: "hold-level".to_string(), ..good_ctx() },
        ),
    ];

    let mut all_expected = true;
    for (label, action, ctx) in &cases {
        match gate.check(*action, ctx) {
            Ok(verdict) => {
                // The good case should approve; every other case should be rejected.
                let expected_ok = *label == "approved-good";
                if verdict.is_approved() != expected_ok {
                    all_expected = false;
                }
                println!("  {label:34} -> {verdict:?}");
            }
            Err(e) => {
                all_expected = false;
                println!("  {label:34} -> ERROR: {e}");
            }
        }
    }

    if all_expected {
        println!("  RESULT: OK — good action approved; every hallucination type was caught.");
    } else {
        println!("  RESULT: UNEXPECTED — a verdict did not match its expectation (see above).");
    }
}
