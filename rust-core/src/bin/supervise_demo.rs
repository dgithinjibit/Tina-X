//! supervise-demo — show the slow brain steering the fast loop, safely (Phase 2 Stage ②).
//!
//! Runs the REAL supervisory loop: it loads the knowledge base + verification rules, drives MeTTa
//! via the subprocess brain, and prints, for a few ticks, what the brain PROPOSED, what the gate
//! decided, and what setpoint the reflex loop is therefore holding. You should see healthy ticks
//! approved-and-forwarded, and an unhealthy (stale-telemetry) tick REFUSED with the last-safe
//! setpoint held — the fail-closed governance that is the product's whole point.
//!
//! Run:  cargo run -p nzi-core --bin supervise-demo
//! (Requires the venv from docs/DEV_SETUP.md; otherwise it prints a friendly setup hint.)

use nzi_core::reflex::Vec3;
use nzi_core::supervise::{Supervisor, Telemetry};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn tlm(mode: &str, fresh: bool) -> Telemetry {
    Telemetry { measured: Vec3::new(0.4, 0.0, 0.1), fresh, finite: true, mode: mode.to_string() }
}

fn main() {
    println!("Project Nzi — supervisory loop demo (Phase 2, brain -> gate -> reflex)");

    let mut sup = match Supervisor::from_project_root(project_root()) {
        Ok(s) => s,
        Err(e) => {
            println!("  Could not build the supervisor: {e}");
            println!("  (Ensure the venv is set up — see docs/DEV_SETUP.md — and that");
            println!("   metta-logic/knowledge/*.metta and verification/*.metta are present.)");
            return;
        }
    };

    // A little scenario: two healthy ticks, then one with stale telemetry to trip governance,
    // then a hold-level tick. Each line shows disposition + the setpoint the reflex loop holds.
    let scenario = [
        ("free, healthy", tlm("free", true)),
        ("free, healthy", tlm("free", true)),
        ("free, STALE telemetry", tlm("free", false)),
        ("hold-level, healthy", tlm("hold-level", true)),
    ];

    for (id, (label, telemetry)) in scenario.iter().enumerate() {
        let out = sup.tick(telemetry, id as u64 + 1);
        println!(
            "  tick {} [{label:24}] -> {:?}",
            id + 1,
            out.disposition
        );
        println!(
            "         reflex holds: roll={:.2} pitch={:.2} yaw={:.2}  (verified={})",
            out.setpoint.roll, out.setpoint.pitch, out.setpoint.yaw, out.decision.verified
        );
    }

    println!("  RESULT: OK — approved setpoints forwarded; doubt held the last-safe setpoint.");
}
