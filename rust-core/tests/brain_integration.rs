//! Integration test: drive the REAL MeTTa via [`SubprocessBrain`].
//!
//! # For a junior dev
//! Unit tests (in `src/brain.rs`) use a fake brain and never touch MeTTa. This test is
//! different: it actually launches `metta-logic/bridge_worker.py` in the project venv and
//! checks the round-trip works.
//!
//! Because not every machine/CI has the venv set up, this test **skips itself gracefully**
//! (prints a note and returns) when MeTTa isn't available, instead of failing. That way the
//! suite stays green everywhere, but proves the real bridge wherever MeTTa is installed.
//!
//! Run it explicitly with output visible:
//!   cargo test -p tina-core --test brain_integration -- --nocapture

use tina_core::brain::{MettaQuery, SubprocessBrain, SymbolicBrain};
use std::path::PathBuf;

/// The project root is the parent of this crate's directory (`rust-core/`).
/// `CARGO_MANIFEST_DIR` is set by Cargo to the crate root at compile time.
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust-core should have a parent directory")
        .to_path_buf()
}

/// Helper: build a brain and skip the test (returning None) if MeTTa isn't set up.
fn brain_or_skip(test_name: &str) -> Option<SubprocessBrain> {
    let brain = SubprocessBrain::from_project_root(project_root());
    if !brain.is_available() {
        eprintln!(
            "SKIP {test_name}: MeTTa venv not found. Run the setup in docs/DEV_SETUP.md \
             (python3 -m venv .venv && .venv/bin/pip install hyperon) to exercise this test."
        );
        return None;
    }
    Some(brain)
}

#[test]
fn arithmetic_round_trip() {
    let Some(brain) = brain_or_skip("arithmetic_round_trip") else { return };

    // Ask MeTTa to add two numbers. The '!' means "evaluate and return the result".
    let out = brain
        .query(&MettaQuery::new("!(+ 1 2)"))
        .expect("query should succeed when MeTTa is available");

    assert_eq!(out.results, vec!["3".to_string()], "1 + 2 should be 3");
}

#[test]
fn safe_to_fly_rule_round_trip() {
    let Some(brain) = brain_or_skip("safe_to_fly_rule_round_trip") else { return };

    // Define a tiny rule, then ask it twice. This mirrors the kind of symbolic decision the
    // slow brain will make (see metta-logic/smoke_test.metta).
    let program = "(= (tol) 12) \
                   (= (safe? $w) (if (<= $w (tol)) True False)) \
                   !(safe? 8) \
                   !(safe? 20)";
    let out = brain
        .query(&MettaQuery::new(program))
        .expect("query should succeed when MeTTa is available");

    // 8 m/s wind is within tolerance -> True; 20 m/s is not -> False.
    assert_eq!(out.results, vec!["True".to_string(), "False".to_string()]);
}
