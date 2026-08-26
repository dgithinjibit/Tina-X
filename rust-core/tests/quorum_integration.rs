//! End-to-end integration test for the honeybee quorum arbiter (bee brain B1–B4).
//!
//! # For a junior dev
//! `src/quorum.rs`'s unit tests use a FakeBrain to check the arbiter's rendering + verdict-parsing
//! in isolation. THIS file drives the REAL chain: it loads the actual
//! `metta-logic/knowledge/quorum.metta`, runs it through MeTTa via the subprocess brain, and
//! confirms the collective-decision behaves — the same cases as `metta-logic/run_quorum_smoke.py`,
//! but reached through the Rust [`QuorumArbiter`] the supervisor/swarm actually call:
//!
//!   * below quorum -> Scout (fail-safe, no blind commit);
//!   * best-supported option past quorum -> Commit to it;
//!   * B2 risk knob: HIGH risk needs far more support than LOW;
//!   * B3/B4 cross-inhibition: at LOW quorum rival stop-signals change the effective winner, while
//!     at HIGH quorum inhibition is OFF (raw support decides).
//!
//! Skips gracefully when the venv isn't set up, like the other integration tests.
//!
//! Run:  cargo test -p tina-core --test quorum_integration -- --nocapture

use tina_core::brain::SubprocessBrain;
use tina_core::quorum::{QuorumArbiter, QuorumVerdict, Risk, Tally};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust-core should have a parent directory")
        .to_path_buf()
}

fn arbiter_or_skip(test_name: &str) -> Option<QuorumArbiter<SubprocessBrain>> {
    let root = project_root();
    if !SubprocessBrain::from_project_root(root.clone()).is_available() {
        eprintln!(
            "SKIP {test_name}: MeTTa venv not found. Run the setup in docs/DEV_SETUP.md \
             to exercise this test."
        );
        return None;
    }
    match QuorumArbiter::from_project_root(&root) {
        Ok(a) => Some(a),
        Err(e) => panic!("venv present but quorum arbiter failed to build: {e}"),
    }
}

fn commit(id: &str, weight: u32) -> QuorumVerdict {
    QuorumVerdict::Commit { id: id.to_string(), weight }
}

#[test]
fn low_risk_lone_option_past_quorum_commits() {
    let Some(a) = arbiter_or_skip("low_risk_lone_option_past_quorum_commits") else { return };
    // Low quorum is 6; support 7, no inhibition -> Commit north@7 (smoke: "low-commit").
    let out = a.decide(Risk::Low, &[Tally::new("north", 7, 0)]).unwrap();
    assert_eq!(out, commit("north", 7));
}

#[test]
fn low_risk_below_quorum_scouts() {
    let Some(a) = arbiter_or_skip("low_risk_below_quorum_scouts") else { return };
    // Support 5 < low quorum 6 -> keep scouting (smoke: "low-below-quorum").
    let out = a.decide(Risk::Low, &[Tally::new("north", 5, 0)]).unwrap();
    assert_eq!(out, QuorumVerdict::Scout);
}

#[test]
fn high_risk_demands_more_support() {
    let Some(a) = arbiter_or_skip("high_risk_demands_more_support") else { return };
    // High quorum is 20: support 7 is plenty for low but not high -> Scout (smoke: "high-needs-more").
    let scout = a.decide(Risk::High, &[Tally::new("north", 7, 0)]).unwrap();
    assert_eq!(scout, QuorumVerdict::Scout);
    // Support 21 crosses the high quorum -> Commit (smoke: "high-commit").
    let commit21 = a.decide(Risk::High, &[Tally::new("north", 21, 0)]).unwrap();
    assert_eq!(commit21, commit("north", 21));
}

#[test]
fn low_risk_cross_inhibition_changes_the_winner() {
    let Some(a) = arbiter_or_skip("low_risk_cross_inhibition_changes_the_winner") else { return };
    // B3 at LOW quorum: 'south' raw 8 but 5 stop-signals -> effective 3; 'north' 7 clean -> wins,
    // and 7 >= quorum 6 -> Commit north@7 (smoke: "low-inhibition-flips-winner").
    let out = a
        .decide(Risk::Low, &[Tally::new("north", 7, 0), Tally::new("south", 8, 5)])
        .unwrap();
    assert_eq!(out, commit("north", 7));
}

#[test]
fn high_risk_inhibition_is_off() {
    let Some(a) = arbiter_or_skip("high_risk_inhibition_is_off") else { return };
    // B4 at HIGH quorum: inhibition OFF, so 'south' (raw 22) beats 'north' (raw 7) despite both
    // carrying 9 stop-signals, and 22 >= quorum 20 -> Commit south@22 (smoke: "high-inhibition-off").
    let out = a
        .decide(Risk::High, &[Tally::new("north", 7, 9), Tally::new("south", 22, 9)])
        .unwrap();
    assert_eq!(out, commit("south", 22));
}
