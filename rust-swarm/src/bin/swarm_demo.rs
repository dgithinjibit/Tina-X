//! swarm-demo — watch a SoNS swarm self-organize, cover a field, and self-heal (Phase 4).
//!
//! Runs the headless swarm and narrates the emergent behaviors that come from neighbor-local rules
//! alone: hierarchy formation (single brain), stigmergic field coverage, and re-election after the
//! brain is killed. No Unity — this is the coordination LOGIC; the 3D view is Phase 7.
//!
//! Run:  cargo run -p nzi-swarm --bin swarm-demo

use nzi_swarm::agent::COVERAGE_TARGET;
use nzi_swarm::Swarm;

fn main() {
    println!("Project Nzi — SoNS swarm demo (Phase 4, headless)");

    let (w, h) = (8, 8);
    let mut swarm = Swarm::grid(w, h);
    println!("  built {}x{} grid = {} agents, neighbor-local (4-neighborhood) comms", w, h, swarm.len());

    // 1. Self-organize the hierarchy.
    swarm.run(30);
    println!(
        "  after 30 ticks: leaders = {:?}  (expect one — the highest id {})",
        swarm.leaders(),
        swarm.len() - 1
    );
    println!(
        "    brain is agent {} at a corner; farthest agent 0 believes it is {} hops away",
        swarm.len() - 1,
        swarm.agent(0).belief().distance
    );

    // 2. Stigmergic coverage.
    let cov = swarm.field().coverage_fraction(COVERAGE_TARGET);
    println!(
        "  field coverage @ target {}: {:.0}%  ({} total marks laid)",
        COVERAGE_TARGET,
        cov * 100.0,
        swarm.field().total_marks()
    );

    // 3. Kill the brain and watch it self-heal.
    let brain = (swarm.len() - 1) as u32;
    swarm.kill(brain);
    let recovery = (nzi_swarm::sons::MAX_HORIZON + 12) as u64;
    println!("  !! killed the brain (agent {brain}) — running {recovery} more ticks (self-heal)...");
    swarm.run(recovery);
    println!(
        "  re-elected leaders = {:?}  (expect the next-highest survivor {})",
        swarm.leaders(),
        brain - 1
    );

    let ok = swarm.leaders() == vec![brain - 1] && cov == 1.0;
    println!(
        "  RESULT: {}",
        if ok {
            "OK — swarm self-organized, covered the field, and self-healed after brain loss."
        } else {
            "REVIEW — an expected emergent property did not hold (see above)."
        }
    );
}
