//! zk-rust demo — prove & verify a safe-to-fly decision, and time it (for ADR 0004).
//!
//! Run:  cargo run -p nzi-zk-rust --release
//! (Release matters: ZK proving is MUCH faster optimized.)

use nzi_zk_rust::{safe_to_fly, SafeToFlyZk};
use std::time::Instant;

fn main() {
    println!("Project Nzi — Rust-native ZK POC (arkworks Groth16 / BN254)");

    // A realistic scenario: private wind 8 m/s, public tolerance 12 m/s -> decision "fly" (1).
    let (wind, tolerance) = (8u64, 12u64);
    let decision = safe_to_fly(wind, tolerance);
    println!("  scenario: wind={wind} (PRIVATE), tolerance={tolerance} (public) -> decision={decision}");

    let t = Instant::now();
    let zk = SafeToFlyZk::setup().expect("setup");
    println!("  setup    : {:?}", t.elapsed());

    let t = Instant::now();
    let proof = zk.prove(wind, tolerance, decision).expect("prove");
    println!("  prove    : {:?}", t.elapsed());

    let t = Instant::now();
    let ok = zk.verify(tolerance, decision, &proof).expect("verify");
    println!("  verify   : {:?}  -> valid={ok}", t.elapsed());

    // Show the verifier truly doesn't learn wind: it only ever received (tolerance, decision).
    println!("  note     : verifier used only public inputs (tolerance, decision); wind stayed hidden.");
    println!("  RESULT: {}", if ok { "OK — decision proven in zero knowledge." } else { "FAIL" });
}
