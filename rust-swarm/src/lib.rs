//! Project Nzi — headless **SoNS swarm** (Phase 4).
//!
//! Scale from one bot to a fleet using a **Self-organizing Nervous System (SoNS)**: a hierarchy
//! that forms at runtime from purely **neighbor-local** interactions, with an interchangeable
//! "brain" (leader) that can be replaced when an agent fails — the swarm self-heals. This is the
//! coordination LOGIC, built and testable **headless** (no Unity); only the 3D visualization of
//! the swarm is deferred to Phase 7.
//!
//! # The one rule that makes it a swarm, not a central controller
//! **No agent ever reads global state.** Every decision an agent makes uses only:
//!   * its own state, and
//!   * messages from its immediate neighbors.
//!
//! The [`Swarm`] orchestrator exists only to *deliver* neighbor-local messages and advance time in
//! lockstep — it never lets an agent peek at the whole field. That constraint is enforced by the
//! module boundaries (an [`agent::Agent`]'s step only receives its inbox), and is what makes the
//! self-organization real rather than centrally faked.
//!
//! # Layers
//! - [`agent`]     — an agent, its neighbor set, and the neighbor-local message type.
//! - [`sons`]      — decentralized hierarchy formation + leader (brain) election/reconfiguration.
//! - [`stigmergy`] — a shared coverage field agents mark locally to allocate field-coverage work.
//! - [`hazard`]    — an optional per-cell flood-risk overlay (from a GEOGLOWS EO feed) that biases
//!   coverage toward the disaster; fuses with stigmergy WITHOUT breaking locality.
//! - [`gnss`]      — GNSS position ingest (NMEA parser + lat/lon→grid mapping) so real receiver
//!   fixes (incl. Galileo) place agents on the grid; the swarm's navigation input.
//! - [`Swarm`]     — the headless orchestrator: builds topology, delivers messages, steps time.

pub mod agent;
pub mod gnss;
pub mod hazard;
pub mod sons;
pub mod stigmergy;

use std::collections::HashMap;

use agent::{Agent, AgentId, Message};
use hazard::HazardField;
use stigmergy::CoverageField;

/// The headless multi-agent simulation. Owns the agents, their topology, and the shared
/// stigmergy field; steps everyone in lockstep delivering ONLY neighbor-local messages.
pub struct Swarm {
    agents: Vec<Agent>,
    /// Undirected adjacency: `neighbors[i]` are the ids agent `i` can talk to. Built once from a
    /// topology; agents never see beyond this.
    neighbors: Vec<Vec<AgentId>>,
    /// The shared environment marking (stigmergy). Agents read/write only their local cell.
    field: CoverageField,
    /// Optional per-cell flood-risk overlay (G4D-RR bridge #6). When present, it biases each cell's
    /// EFFECTIVE coverage level so the swarm concentrates effort where the disaster is — without any
    /// agent seeing the global map (locality preserved; see [`hazard`]). `None` = pure stigmergy.
    hazard: Option<HazardField>,
    /// Which agents are alive. A "killed" agent stops sending/acting; the swarm must self-heal
    /// around it (P4.4). Kept as a parallel flag vec so ids stay stable indices.
    alive: Vec<bool>,
    /// Monotonic tick counter.
    tick: u64,
}

impl Swarm {
    /// Build a swarm of `n` agents on a `width`×`height` grid torus-free lattice, wiring each
    /// agent to its 4-neighborhood (von Neumann). The grid is the field to be covered; agent `i`
    /// starts at cell `(i % width, i / width)`.
    ///
    /// The lattice is just a concrete, testable topology — the coordination logic only assumes
    /// "each agent has some neighbors", so other topologies drop in without touching `sons`.
    pub fn grid(width: usize, height: usize) -> Self {
        let n = width * height;
        let mut agents = Vec::with_capacity(n);
        let mut neighbors: Vec<Vec<AgentId>> = Vec::with_capacity(n);
        for i in 0..n {
            let (x, y) = (i % width, i / width);
            agents.push(Agent::new(i as AgentId, x, y));
            let mut nbrs = Vec::with_capacity(4);
            for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                    nbrs.push((ny as usize * width + nx as usize) as AgentId);
                }
            }
            neighbors.push(nbrs);
        }
        Self {
            agents,
            neighbors,
            field: CoverageField::new(width, height),
            hazard: None,
            alive: vec![true; n],
            tick: 0,
        }
    }

    /// Attach a flood-risk overlay (G4D-RR bridge #6) so the swarm biases coverage toward
    /// high-risk cells. The field's dimensions MUST match the swarm's grid; mismatch panics (a
    /// caller programming error). Builder-style so a server handler can do
    /// `Swarm::grid(w, h).with_hazard(field)`. `None`/unattached = pure stigmergy (unchanged).
    pub fn with_hazard(mut self, hazard: HazardField) -> Self {
        assert_eq!(
            (hazard.width(), hazard.height()),
            (self.field.width(), self.field.height()),
            "hazard field dimensions must match the swarm grid"
        );
        self.hazard = Some(hazard);
        self
    }

    /// Read-only view of the attached hazard overlay, if any (tests/telemetry).
    pub fn hazard(&self) -> Option<&HazardField> {
        self.hazard.as_ref()
    }

    /// Number of agents (including any that have been killed — ids stay stable).
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Current tick.
    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    /// Read-only view of an agent (for tests/telemetry/inspection — NOT used by agents).
    pub fn agent(&self, id: AgentId) -> &Agent {
        &self.agents[id as usize]
    }

    /// Whether an agent is currently alive.
    pub fn is_alive(&self, id: AgentId) -> bool {
        self.alive[id as usize]
    }

    /// Kill an agent (simulate hardware failure). It stops sending and acting; the swarm must
    /// reconfigure around it (resilience — P4.4).
    pub fn kill(&mut self, id: AgentId) {
        self.alive[id as usize] = false;
    }

    /// Revive / add an agent back into the swarm (rejoin after failure — P4.4).
    pub fn revive(&mut self, id: AgentId) {
        self.alive[id as usize] = true;
    }

    /// The stigmergy field (read-only view for tests/telemetry).
    pub fn field(&self) -> &CoverageField {
        &self.field
    }

    /// Advance the whole swarm by one tick with STRICTLY neighbor-local information flow:
    ///   1. every alive agent produces outbound messages from its own state only,
    ///   2. messages are delivered to each recipient's inbox (only along the topology edges),
    ///   3. every alive agent consumes its inbox + deposits pheromone on its local stigmergy cell,
    ///   4. the whole pheromone field evaporates one tick (ACO decay — the "useful forgetting"
    ///      that lets coverage re-flow to stale gaps; see `stigmergy` A1/A2).
    ///
    /// Steps (1) and (3) are separated so all agents act on the SAME tick's snapshot — no agent
    /// sees another's within-tick update, which would smuggle in global/instantaneous knowledge.
    /// Evaporation (4) runs AFTER deposits, matching ACO's deposit-then-evaporate update order.
    pub fn step(&mut self) {
        self.tick += 1;

        // 1. Collect outbound messages (id -> messages it wants to send to neighbors).
        //    An agent only knows its neighbor ids; it addresses messages to them, nothing else.
        let mut outbox: Vec<(AgentId, Message)> = Vec::new();
        for i in 0..self.agents.len() {
            if !self.alive[i] {
                continue;
            }
            let nbrs = &self.neighbors[i];
            for msg in self.agents[i].produce(nbrs) {
                outbox.push((msg.to, msg.message));
            }
        }

        // 2. Deliver: route each message into the recipient's inbox, dropping any to dead agents.
        let mut inboxes: HashMap<AgentId, Vec<Message>> = HashMap::new();
        for (to, msg) in outbox {
            if self.alive[to as usize] {
                inboxes.entry(to).or_default().push(msg);
            }
        }

        // 3. Consume + act locally. Each agent updates hierarchy state and marks its cell.
        for i in 0..self.agents.len() {
            if !self.alive[i] {
                continue;
            }
            let inbox = inboxes.remove(&(i as AgentId)).unwrap_or_default();
            let (x, y) = self.agents[i].position();
            // The agent reads ONE scalar for its own cell — the physical pheromone, biased down by
            // this cell's flood risk when a hazard overlay is attached (G4D-RR bridge #6). High risk
            // → reads as under-covered → keeps servicing. No overlay → the raw pheromone (unchanged).
            let phero = self.field.level(x, y);
            let local_level = match &self.hazard {
                Some(h) => h.effective_level(x, y, phero),
                None => phero,
            };
            self.agents[i].consume(inbox, local_level);
            // Stigmergy: an agent that is actively covering marks (deposits pheromone on) its cell.
            if self.agents[i].is_covering() {
                self.field.mark(x, y);
            }
        }

        // 4. Evaporate the shared pheromone field one tick (ACO decay). Done once, after all
        //    deposits, so a covered cell's trail fades over time and coverage re-flows to gaps.
        self.field.evaporate();
    }

    /// Run `ticks` steps. Convenience for demos/tests.
    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step();
        }
    }

    /// The set of agents that currently believe they are a leader ("brain"). In a healthy,
    /// converged swarm on a connected topology this is exactly one per connected component.
    /// (Inspection helper — agents themselves only know their LOCAL role.)
    pub fn leaders(&self) -> Vec<AgentId> {
        (0..self.agents.len())
            .filter(|&i| self.alive[i] && self.agents[i].is_leader())
            .map(|i| i as AgentId)
            .collect()
    }

    /// Neighbor-local **support/inhibition tallies** for each leader candidate — the cheap swarm
    /// signal the honeybee quorum arbiter deliberates over (bee brain B1–B4; see
    /// `nzi_core::quorum`). "Which brain does the group commit to?" is a quorum decision, and this
    /// is the raw evidence for it.
    ///
    /// For every id that at least one alive agent currently believes leads:
    ///   * `support`    = number of alive agents that believe in THIS candidate;
    ///   * `inhibition` = number of alive agents that believe in a *higher-id* rival — the
    ///     cross-inhibition pressure against committing to this candidate (honeybee stop-signals:
    ///     scouts backing a stronger site inhibit commitment to weaker ones).
    ///
    /// In a fully converged, connected swarm this returns a SINGLE candidate with full support and
    /// zero inhibition (→ an easy quorum Commit). Before convergence, or in a partitioned swarm,
    /// several candidates appear with competing support and non-zero inhibition — exactly the
    /// contested case cross-inhibition exists to resolve. Returned sorted by id for determinism.
    ///
    /// This is an INSPECTION helper (it aggregates global belief for an external arbiter); no agent
    /// reads it. It is plain data — the caller maps each `(id, support, inhibition)` to a
    /// `nzi_core::quorum::Tally` (the swarm crate stays dependency-free).
    pub fn candidate_tallies(&self) -> Vec<CandidateTally> {
        // Tally believed-leader ids across alive agents.
        let mut believers: std::collections::BTreeMap<AgentId, u32> = std::collections::BTreeMap::new();
        for i in 0..self.agents.len() {
            if self.alive[i] {
                *believers.entry(self.agents[i].belief().leader).or_insert(0) += 1;
            }
        }
        // For each candidate, inhibition = believers in any strictly-higher-id candidate.
        believers
            .iter()
            .map(|(&id, &support)| {
                let inhibition = believers
                    .iter()
                    .filter(|(&other, _)| other > id)
                    .map(|(_, &n)| n)
                    .sum();
                CandidateTally { id, support, inhibition }
            })
            .collect()
    }

    /// A plain-data snapshot of the swarm for external consumers (a dashboard / API). This is an
    /// INSPECTION view — it deliberately exposes global state that no agent may read, precisely
    /// because it's for a human observer, not for an agent's decision. No serde here so the crate
    /// stays dependency-light; the server maps this into its own serializable DTO.
    pub fn snapshot(&self, service_target: u32) -> SwarmSnapshot {
        let leaders = self.leaders();
        // The single believed brain for display: the highest-id current leader (one in a converged,
        // connected swarm). None only if every agent is dead.
        let leader = leaders.iter().copied().max();
        let agents = (0..self.agents.len())
            .map(|i| {
                let (x, y) = self.agents[i].position();
                AgentSnapshot {
                    id: i as AgentId,
                    x,
                    y,
                    alive: self.alive[i],
                    is_leader: self.alive[i] && self.agents[i].is_leader(),
                    believes_leader: self.agents[i].belief().leader,
                    distance_to_leader: self.agents[i].belief().distance,
                    covering: self.agents[i].is_covering(),
                }
            })
            .collect();
        // Per-cell pheromone (live, decaying) and cumulative coverage (monotone), row-major.
        let w = self.field.width();
        let h = self.field.height();
        let mut pheromone = Vec::with_capacity(w * h);
        let mut coverage = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                pheromone.push(self.field.level(x, y));
                coverage.push(self.field.coverage_count(x, y));
            }
        }
        // Flood-risk overlay (G4D-RR bridge #6): row-major risk grid + mean, or empty/0 when no
        // overlay is attached (pure stigmergy). Kept alongside the field so the dashboard can show
        // WHERE the swarm is being pulled and HOW threatened the whole area is.
        let (hazard_risk, hazard_mean) = match &self.hazard {
            Some(hz) => (hz.levels(), hz.mean_risk()),
            None => (Vec::new(), 0.0),
        };
        SwarmSnapshot {
            width: w,
            height: h,
            tick: self.tick,
            leader,
            leader_count: leaders.len(),
            coverage_fraction: self.field.coverage_fraction(service_target),
            rho: self.field.rho(),
            agents,
            pheromone,
            coverage,
            hazard_risk,
            hazard_mean,
        }
    }
}

/// A point-in-time, plain-data view of the whole swarm (for a dashboard/API). Global by design —
/// this is the observer's view, never an agent's.
#[derive(Clone, Debug)]
pub struct SwarmSnapshot {
    pub width: usize,
    pub height: usize,
    pub tick: u64,
    /// The displayed brain (highest-id current leader), or None if all agents are dead.
    pub leader: Option<AgentId>,
    /// How many agents currently believe they lead — 1 in a healthy converged connected swarm.
    pub leader_count: usize,
    /// Fraction of the field serviced to the cumulative target, in [0,1].
    pub coverage_fraction: f64,
    /// The stigmergy evaporation rate ρ in use.
    pub rho: f64,
    pub agents: Vec<AgentSnapshot>,
    /// Row-major live pheromone per cell (decaying).
    pub pheromone: Vec<f64>,
    /// Row-major cumulative coverage count per cell (monotone).
    pub coverage: Vec<u32>,
    /// Row-major flood-risk per cell in `[0,1]` from the attached hazard overlay (G4D-RR bridge #6),
    /// or EMPTY when no overlay is attached (pure stigmergy). Same layout as `pheromone`/`coverage`.
    pub hazard_risk: Vec<f64>,
    /// Mean flood risk across the field in `[0,1]` (0 when no overlay) — a headline threat metric.
    pub hazard_mean: f64,
}

/// A leader candidate's neighbor-local support/inhibition tally — the swarm-native evidence the
/// honeybee quorum arbiter (`nzi_core::quorum`) deliberates over. Plain data: the caller maps it to
/// a `nzi_core::quorum::Tally` (keeping this crate dependency-free).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateTally {
    /// The candidate leader's agent id.
    pub id: AgentId,
    /// Alive agents that currently believe in THIS candidate.
    pub support: u32,
    /// Alive agents that believe in a strictly-higher-id rival (cross-inhibition pressure).
    pub inhibition: u32,
}

/// One agent's inspectable state in a [`SwarmSnapshot`].
#[derive(Clone, Copy, Debug)]
pub struct AgentSnapshot {
    pub id: AgentId,
    pub x: usize,
    pub y: usize,
    pub alive: bool,
    pub is_leader: bool,
    pub believes_leader: AgentId,
    pub distance_to_leader: u32,
    pub covering: bool,
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::hazard::HazardField;
    use crate::stigmergy::SERVICE_TARGET;

    #[test]
    fn snapshot_has_no_hazard_by_default() {
        // Without an overlay attached, the swarm behaves exactly as before: empty risk grid, mean 0.
        let mut swarm = Swarm::grid(4, 4);
        swarm.run(10);
        let snap = swarm.snapshot(SERVICE_TARGET);
        assert!(snap.hazard_risk.is_empty(), "no overlay -> empty risk grid");
        assert_eq!(snap.hazard_mean, 0.0);
        assert!(swarm.hazard().is_none());
    }

    #[test]
    fn snapshot_exposes_the_attached_hazard_overlay() {
        // A 2x2 overlay with one hot corner should surface row-major in the snapshot with the right
        // mean, matching the pheromone/coverage layout (cell (x,y) at index y*width + x).
        let risk = vec![0.0, 0.0, 0.0, 1.0]; // only cell (1,1) hot
        let hz = HazardField::from_levels(2, 2, risk.clone(), hazard::DEFAULT_HAZARD_BIAS);
        let mut swarm = Swarm::grid(2, 2).with_hazard(hz);
        swarm.run(5);
        let snap = swarm.snapshot(SERVICE_TARGET);
        assert_eq!(snap.hazard_risk, risk);
        assert_eq!(snap.hazard_mean, 0.25);
    }

    #[test]
    fn hazard_keeps_a_high_risk_cell_covered_longer_than_a_safe_one() {
        // The behavioral payoff: give a 1x2 grid one max-risk cell and one safe cell, run long
        // enough for pheromone to build then evaporate. The high-risk cell must accumulate strictly
        // MORE cumulative coverage than the safe cell — the swarm concentrated effort on the hazard.
        let risk = vec![1.0, 0.0]; // cell (0,0) max risk, cell (1,0) safe
        let hz = HazardField::from_levels(2, 1, risk, hazard::DEFAULT_HAZARD_BIAS);
        let mut swarm = Swarm::grid(2, 1).with_hazard(hz);
        swarm.run(40);
        let hot = swarm.field().coverage_count(0, 0);
        let safe = swarm.field().coverage_count(1, 0);
        assert!(
            hot > safe,
            "high-risk cell should be serviced more (hot={hot}, safe={safe})"
        );
    }

    #[test]
    #[should_panic(expected = "hazard field dimensions must match")]
    fn attaching_a_mismatched_hazard_panics() {
        let hz = HazardField::new(3, 3);
        let _ = Swarm::grid(4, 4).with_hazard(hz); // 3x3 overlay on a 4x4 grid -> panic
    }

    #[test]
    fn snapshot_reflects_a_converged_swarm() {
        let mut swarm = Swarm::grid(4, 4);
        swarm.run(20);
        let snap = swarm.snapshot(SERVICE_TARGET);
        assert_eq!(snap.width, 4);
        assert_eq!(snap.height, 4);
        assert_eq!(snap.agents.len(), 16);
        assert_eq!(snap.pheromone.len(), 16);
        assert_eq!(snap.coverage.len(), 16);
        // One converged leader = the highest id (15) on a connected grid.
        assert_eq!(snap.leader, Some(15));
        assert_eq!(snap.leader_count, 1);
        // The field got serviced, and ρ is the field's rate.
        assert!(snap.coverage_fraction > 0.0);
        assert!(snap.rho > 0.0);
    }

    #[test]
    fn snapshot_leader_is_none_when_all_dead() {
        let mut swarm = Swarm::grid(2, 2);
        for id in 0..4u32 {
            swarm.kill(id);
        }
        let snap = swarm.snapshot(SERVICE_TARGET);
        assert_eq!(snap.leader, None);
        assert_eq!(snap.leader_count, 0);
    }

    #[test]
    fn converged_swarm_yields_one_uncontested_candidate() {
        // Once converged on a connected grid, every agent believes the highest id (15) -> a single
        // candidate with full support and zero inhibition: the easy-quorum Commit case.
        let mut swarm = Swarm::grid(4, 4);
        swarm.run(20);
        let tallies = swarm.candidate_tallies();
        assert_eq!(tallies.len(), 1, "converged swarm has one believed leader");
        assert_eq!(tallies[0].id, 15);
        assert_eq!(tallies[0].support, 16, "all 16 agents believe in it");
        assert_eq!(tallies[0].inhibition, 0, "no higher-id rival exists");
    }

    #[test]
    fn fresh_swarm_yields_contested_candidates_with_inhibition() {
        // Before ANY gossip, each agent believes only in itself: N candidates, each supported by 1,
        // and each inhibited by every higher-id candidate (cross-inhibition pressure). This is the
        // contested pre-convergence case the quorum arbiter's cross-inhibition is built to resolve.
        let swarm = Swarm::grid(2, 2); // ids 0..=3, no ticks run
        let tallies = swarm.candidate_tallies();
        assert_eq!(tallies.len(), 4);
        // Sorted by id; support is 1 each; inhibition = count of higher ids (3,2,1,0).
        assert_eq!(tallies[0], CandidateTally { id: 0, support: 1, inhibition: 3 });
        assert_eq!(tallies[3], CandidateTally { id: 3, support: 1, inhibition: 0 });
    }

    #[test]
    fn dead_agents_do_not_count_toward_tallies() {
        // Killed agents neither support nor inhibit — only ALIVE beliefs form the evidence.
        let mut swarm = Swarm::grid(2, 2);
        swarm.kill(3); // remove the would-be top candidate before it gossips
        let tallies = swarm.candidate_tallies();
        assert_eq!(tallies.len(), 3, "id 3 is dead, so only 0,1,2 are candidates");
        assert!(tallies.iter().all(|t| t.id != 3));
    }
}
