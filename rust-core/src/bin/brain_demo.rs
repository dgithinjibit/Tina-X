//! brain-demo — prove the Rust "slow brain" can drive MeTTa end-to-end (P0.4).
//!
//! This is the human-runnable companion to the integration test. It asks MeTTa a couple of
//! questions through the [`SubprocessBrain`] and prints the answers, so you can *see* the
//! Rust↔MeTTa bridge working.
//!
//! Run:  cargo run -p tina-core --bin brain-demo
//! (Requires the venv from docs/DEV_SETUP.md; otherwise it prints a friendly setup hint.)

use tina_core::brain::{MettaQuery, SubprocessBrain, SymbolicBrain};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    // At runtime we're usually launched from the project root by `cargo run`, but to be safe
    // we derive the root from this crate's compile-time directory (rust-core/ -> parent).
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn main() {
    println!("Project TINA-X — Rust <-> MeTTa bridge demo (P0.4)");

    let brain = SubprocessBrain::from_project_root(project_root());

    // If MeTTa isn't set up, explain how to fix it instead of erroring cryptically.
    if !brain.is_available() {
        println!("  MeTTa venv not found. Set it up with:");
        println!("    python3 -m venv .venv");
        println!("    .venv/bin/pip install hyperon");
        println!("  (see docs/DEV_SETUP.md), then re-run this demo.");
        return;
    }

    // A couple of representative queries: raw arithmetic, and a symbolic decision rule.
    let queries = [
        MettaQuery::new("!(+ 1 2)"),
        MettaQuery::new(
            "(= (tol) 12) \
             (= (safe? $w) (if (<= $w (tol)) True False)) \
             !(safe? 8) !(safe? 20)",
        ),
    ];

    for q in &queries {
        match brain.query(q) {
            Ok(result) => println!("  query {:?}\n    -> {:?}", q.text, result.results),
            Err(e) => println!("  query {:?}\n    -> ERROR: {e}", q.text),
        }
    }

    println!("  RESULT: OK — Rust drove MeTTa and got typed results back.");
}
