//! zk-rust demo — prove & verify a safe-to-fly decision, and time it (for ADR 0004).
//!
//! Run:  cargo run -p tina-zk-rust --release
//! (Release matters: ZK proving is MUCH faster optimized.)

use tina_zk_rust::{safe_to_fly, SafeToFlyZk};
use std::time::Instant;

fn main() {
    println!("Project TINA-X — Rust-native ZK POC (arkworks Groth16 / BN254)");

    // A realistic scenario: private wind 8 m/s + AUTHENTICATED (OSNMA) position, public tolerance
    // 12 m/s -> decision "fly" (1). Fly needs BOTH safe wind AND authenticated signals (bridge #1).
    let (wind, tolerance, authenticated) = (8u64, 12u64, true);
    let decision = safe_to_fly(wind, tolerance, authenticated);
    println!("  scenario: wind={wind} (PRIVATE), OSNMA authenticated={authenticated} (PRIVATE), tolerance={tolerance} (public) -> decision={decision}");

    let t = Instant::now();
    let zk = SafeToFlyZk::setup().expect("setup");
    println!("  setup    : {:?}", t.elapsed());

    let t = Instant::now();
    let proof = zk.prove(wind, authenticated, tolerance, decision).expect("prove");
    println!("  prove    : {:?}", t.elapsed());

    let t = Instant::now();
    let ok = zk.verify(tolerance, decision, &proof).expect("verify");
    println!("  verify   : {:?}  -> valid={ok}", t.elapsed());

    // Show the verifier truly doesn't learn wind OR the raw auth data: it only ever received
    // (tolerance, decision). "Positioned by authenticated signals" is proven without revealing them.
    println!("  note     : verifier used only public inputs (tolerance, decision); wind + OSNMA status stayed hidden.");
    println!("  RESULT: {}", if ok { "OK — decision proven in zero knowledge." } else { "FAIL" });
}
