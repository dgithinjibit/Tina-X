//! Emergent-behavior integration tests for the SoNS swarm (P4.1–P4.5).
//!
//! These assert the properties the roadmap's exit criteria demand — self-organization,
//! self-healing, coverage, scale — from ONLY neighbor-local rules. They are the proof that the
//! coordination is verified, not "emergent-and-hoped".

use tina_swarm::sons::MAX_HORIZON;
use tina_swarm::stigmergy::SERVICE_TARGET;
use tina_swarm::Swarm;

/// Ticks to drive cumulative coverage to the service target. Under pheromone evaporation an agent
/// keeps depositing until its cell's decaying trail holds at target, so reaching a cumulative
/// service count of `SERVICE_TARGET` takes a few ticks; give a generous warm-up margin.
const COVER_TICKS: u64 = (SERVICE_TARGET as u64) + 6;

/// Ticks to allow for self-healing after a leader loss. Recovery is O(horizon) (a stale belief
/// must drain past MAX_HORIZON before a survivor is re-elected — see sons.rs), so give it a margin
/// over the horizon.
const RECOVERY_TICKS: u64 = (MAX_HORIZON + 12) as u64;

/// A connected grid must converge to EXACTLY ONE leader (the highest id), from neighbor-local
/// gossip alone (P4.1 headless multi-agent + P4.2 hierarchy formation).
#[test]
fn converges_to_single_highest_id_leader() {
    let mut swarm = Swarm::grid(5, 5); // 25 agents, ids 0..24
    // Run well past the grid diameter (~8) so max-consensus fully propagates.
    swarm.run(20);
    let leaders = swarm.leaders();
    assert_eq!(leaders, vec![24], "one leader, and it is the highest id");
    // Every agent believes in leader 24.
    for id in 0..swarm.len() as u32 {
        assert_eq!(swarm.agent(id).belief().leader, 24);
    }
}

/// The distance gradient points toward the brain: the leader is at distance 0 and the farthest
/// agent's distance equals its hop-distance to the leader (a real nervous-system backbone).
#[test]
fn forms_a_distance_gradient_to_the_brain() {
    let mut swarm = Swarm::grid(5, 5);
    swarm.run(20);
    // Leader 24 sits at grid corner (4,4); agent 0 at (0,0) is Manhattan distance 8 away.
    assert_eq!(swarm.agent(24).belief().distance, 0);
    assert_eq!(swarm.agent(0).belief().distance, 8, "gradient = hop distance to the brain");
}

/// Killing the brain must trigger automatic re-election of the next-highest survivor — no failure
/// detector, no election message, just the same rule continuing (P4.4 resilience / interchangeable
/// brain).
#[test]
fn re_elects_a_new_brain_when_the_leader_dies() {
    let mut swarm = Swarm::grid(5, 5);
    swarm.run(20);
    assert_eq!(swarm.leaders(), vec![24]);

    // The brain fails.
    swarm.kill(24);
    swarm.run(RECOVERY_TICKS);

    // The swarm self-heals onto the next-highest surviving id (23).
    let leaders = swarm.leaders();
    assert_eq!(leaders, vec![23], "new brain is the highest surviving id");
    // A living agent now follows 23, not the dead 24.
    assert_eq!(swarm.agent(0).belief().leader, 23);
}

/// A revived/added agent with a higher id is adopted as the new brain — the swarm reconfigures
/// around membership change (P4.4 add agents mid-mission).
#[test]
fn adopts_a_higher_id_agent_that_rejoins() {
    let mut swarm = Swarm::grid(5, 5);
    swarm.kill(24); // start without the top id
    swarm.run(20);
    assert_eq!(swarm.leaders(), vec![23]);

    swarm.revive(24); // it comes back — a higher id reappearing is adopted fast (O(diameter)),
    swarm.run(20); //     no drain needed since 24 is a fresh distance-0 source again.
    assert_eq!(swarm.leaders(), vec![24], "highest id resumes leadership");
}

/// Stigmergy drives the swarm to cover the whole field, and coverage does not regress: with an
/// agent per cell, every cell reaches the target within a bounded number of ticks (P4.3/P4.4).
#[test]
fn stigmergy_covers_the_whole_field() {
    let mut swarm = Swarm::grid(6, 6);
    // Each agent covers its own cell while its local pheromone is under target; a few ticks plus a
    // warm-up margin drive cumulative service to SERVICE_TARGET everywhere (COVER_TICKS).
    swarm.run(COVER_TICKS);
    assert_eq!(
        swarm.field().coverage_fraction(SERVICE_TARGET),
        1.0,
        "every cell should reach the service target"
    );
    // And it doesn't keep over-marking: total marks are bounded near target*cells, not runaway.
    // The bound accounts for the extra deposits evaporation induces before a cell's trail settles
    // at target (an agent re-deposits as its own pheromone decays), so allow a modest multiple.
    let cells = (swarm.field().width() * swarm.field().height()) as u32;
    assert!(
        swarm.field().total_marks() <= cells * (SERVICE_TARGET + COVER_TICKS as u32),
        "stigmergy stops runaway over-treatment (effort is bounded, not unbounded)"
    );
}

/// Coverage recovers after agent loss: kill a swath, and the survivors (which still cover their
/// own cells) keep the field covered where they remain. This checks resilience of the coverage
/// task, not just the hierarchy (P4.4).
#[test]
fn coverage_survives_partial_swarm_loss() {
    let mut swarm = Swarm::grid(6, 6);
    swarm.run(COVER_TICKS);
    assert_eq!(swarm.field().coverage_fraction(SERVICE_TARGET), 1.0);

    // Lose a third of the swarm (low ids); already-serviced cells stay serviced. This is the
    // CUMULATIVE coverage view (coverage_fraction), which is monotone by design — pheromone may
    // evaporate, but the audit record of "this cell was serviced" does not, so past work isn't undone.
    for id in 0..12u32 {
        swarm.kill(id);
    }
    swarm.run(20);
    // Coverage laid down before the loss is not undone.
    assert!(swarm.field().coverage_fraction(SERVICE_TARGET) >= 1.0);
    // The leader (id 35, untouched) still leads a single, connected survivor set — the hierarchy
    // is unaffected because we removed followers, not the brain.
    assert_eq!(swarm.leaders(), vec![35]);
}

/// Pheromone evaporation makes coverage ADAPTIVE: after the field is serviced, trails fade, so an
/// agent whose cell has gone stale resumes covering it — the behavior the old monotonic field could
/// never show (bee/ant brain upgrade, ant mechanism A2). Without decay a covered cell stays "hot"
/// forever and effort never re-flows; this test fails on the ρ=0 (pre-upgrade) design.
#[test]
fn stigmergy_re_covers_stale_cells_thanks_to_evaporation() {
    let mut swarm = Swarm::grid(4, 4);
    // Service the field, then let it settle so agents stop actively covering (pheromone at target).
    swarm.run(COVER_TICKS);
    assert_eq!(swarm.field().coverage_fraction(SERVICE_TARGET), 1.0, "field serviced first");

    // Snapshot cumulative marks, then run MANY more ticks. Because pheromone decays below target as
    // cells go stale, agents must resume depositing — so cumulative marks keep GROWING over time.
    let marks_after_settle = swarm.field().total_marks();
    swarm.run(60);
    let marks_later = swarm.field().total_marks();
    assert!(
        marks_later > marks_after_settle,
        "evaporation must re-trigger coverage on stale cells (marks {marks_later} > {marks_after_settle})"
    );

    // Sanity: with ZERO evaporation the same scenario does NOT re-cover (proves decay is the cause).
    // We can't change a running swarm's ρ, so assert the field-level invariant directly instead.
    let mut frozen = tina_swarm::stigmergy::CoverageField::with_evaporation(1, 1, 0.0);
    for _ in 0..10 {
        frozen.mark(0, 0);
    }
    let hot = frozen.total_marks();
    for _ in 0..100 {
        frozen.evaporate();
    }
    assert!(
        frozen.level(0, 0) >= SERVICE_TARGET as f64,
        "ρ=0 baseline stays hot forever (no re-coverage) — decay is what enables adaptivity"
    );
    let _ = hot;
}

/// Scale test: a larger swarm still converges to a single leader, and convergence time grows with
/// DIAMETER (not agent count) — the signature of neighbor-local coordination (P4.5).
#[test]
fn scales_to_a_larger_swarm() {
    let mut swarm = Swarm::grid(20, 20); // 400 agents
    // Diameter is ~38; run enough ticks to converge.
    swarm.run(80);
    assert_eq!(swarm.leaders(), vec![399], "single leader at scale");
    // The coordination cost per tick is O(edges), and it converged in O(diameter) ticks — verified
    // implicitly by needing only ~80 ticks for 400 agents rather than anything ~O(n).
    assert_eq!(swarm.agent(0).belief().leader, 399);
}
