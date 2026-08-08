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
//! - [`Swarm`]     — the headless orchestrator: builds topology, delivers messages, steps time.

pub mod agent;
pub mod sons;
pub mod stigmergy;

use std::collections::HashMap;

use agent::{Agent, AgentId, Message};
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
            alive: vec![true; n],
            tick: 0,
        }
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
    ///   3. every alive agent consumes its inbox + marks its local stigmergy cell.
    ///
    /// Steps (1) and (3) are separated so all agents act on the SAME tick's snapshot — no agent
    /// sees another's within-tick update, which would smuggle in global/instantaneous knowledge.
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
            self.agents[i].consume(inbox, self.field.level(x, y));
            // Stigmergy: an agent that is actively covering marks its current cell.
            if self.agents[i].is_covering() {
                self.field.mark(x, y);
            }
        }
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
}
