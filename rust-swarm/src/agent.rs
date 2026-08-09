//! An agent and its neighbor-local messaging (Phase 4).
//!
//! An [`Agent`] knows only its own state and whatever its neighbors told it last tick. It never
//! sees global state. Its two entry points are:
//!   * [`Agent::produce`] — emit messages to neighbors from own state only, and
//!   * [`Agent::consume`] — fold the inbox + local stigmergy level into new state.
//!
//! The self-organizing hierarchy (SoNS) is carried in these messages; the actual election logic
//! lives in [`crate::sons`] and is applied here.

use crate::sons::LeaderBelief;
use crate::stigmergy::COVERAGE_TARGET;

/// Stable agent identifier = its index in the swarm's agent vector.
pub type AgentId = u32;

/// A message from one agent to one neighbor. Neighbor-local by construction: `to` must be a
/// neighbor of the sender (the swarm only routes along topology edges).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// Recipient (a neighbor of the sender).
    pub to: AgentId,
    /// The payload.
    pub message: Message,
}

/// What agents actually say to each other. Deliberately tiny: everything the SoNS hierarchy needs
/// travels as a gossiped leader-belief. Task/coverage coordination rides on the same channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    /// The sender (so the receiver can attribute the belief).
    pub from: AgentId,
    /// The sender's current belief about who the leader ("brain") is, and how far away it is.
    pub leader: LeaderBelief,
}

/// One swarm agent.
#[derive(Clone, Debug)]
pub struct Agent {
    id: AgentId,
    /// Grid position (the field cell this agent occupies). Fixed for Phase 4 — mobility is a
    /// later concern; coordination + coverage logic doesn't need movement to be proven.
    x: usize,
    y: usize,
    /// This agent's current belief about the leader of its component. Updated every tick from
    /// neighbor messages via the SoNS rule. Initialized to "I am my own leader, distance 0".
    belief: LeaderBelief,
    /// Whether this agent is actively covering (marking) its cell this tick. An agent covers when
    /// its local cell is under-covered — a stigmergic, purely local decision.
    covering: bool,
}

impl Agent {
    /// A fresh agent at grid cell `(x, y)`, initially believing itself the leader.
    pub fn new(id: AgentId, x: usize, y: usize) -> Self {
        Self { id, x, y, belief: LeaderBelief::own(id), covering: false }
    }

    pub fn id(&self) -> AgentId {
        self.id
    }

    pub fn position(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    /// This agent's current belief about who leads its component.
    pub fn belief(&self) -> LeaderBelief {
        self.belief
    }

    /// Does this agent currently believe IT is the leader (the transient "brain")?
    pub fn is_leader(&self) -> bool {
        self.belief.leader == self.id
    }

    /// Alias used by the swarm's inspection helper.
    pub fn is_covering(&self) -> bool {
        self.covering
    }

    /// STEP part 1: produce outbound messages from own state only. Every agent gossips its current
    /// leader-belief to each neighbor; that gossip is what makes the hierarchy self-organize.
    pub fn produce(&self, neighbors: &[AgentId]) -> Vec<Envelope> {
        let message = Message { from: self.id, leader: self.belief };
        neighbors.iter().map(|&to| Envelope { to, message }).collect()
    }

    /// STEP part 2: fold the inbox + local stigmergy level into new state.
    ///
    /// - Hierarchy: adopt the "best" leader belief among {own, all neighbors'} per the SoNS rule
    ///   ([`LeaderBelief::merge`]). Because the rule is a deterministic max-consensus, beliefs
    ///   converge to a single leader per connected component with no central coordinator — and if
    ///   that leader dies, the remaining agents re-converge on a new one (self-healing brain).
    /// - Coverage: decide whether to cover this tick from the LOCAL cell level only (stigmergy).
    pub fn consume(&mut self, inbox: Vec<Message>, local_level: f64) {
        // Loop-free distributed gradient election. An agent adopts a remote leader ONLY through a
        // neighbor that is STRICTLY CLOSER to that leader than the agent could otherwise be — i.e.
        // it takes each neighbor's reported distance, adds one hop, and keeps the best. The leader
        // itself is the sole distance-0 source of its own id. This single rule gives us both
        // properties for free, with no horizon/TTL and no simultaneous reset:
        //
        //   * Convergence: the highest id's gradient advances one hop per tick (O(diameter)), and
        //     every agent ends at its true hop-distance to the brain.
        //   * Self-healing: when the brain dies it stops being a distance-0 source. Its former
        //     followers can now only hear it via neighbors whose distance is >= theirs, which the
        //     "+1 hop" makes strictly worse — so the belief cannot be sustained and collapses in
        //     O(diameter) ticks. Each agent then falls back to the best LIVE gradient it still
        //     hears, or to itself, and the next-highest id wins by the same rule.
        //
        // We do NOT carry our own prior belief forward: a self-sourced belief is exactly the cycle
        // that would pin a dead leader in place. Belief is rebuilt each tick from `own(id)` plus
        // this tick's neighbor gossip only.
        let mut best = LeaderBelief::own(self.id);
        for msg in &inbox {
            // Adopt the neighbor's leader, one hop further away — but reject a belief that has
            // travelled past the freshness horizon. A LIVE leader keeps its gradient short (its
            // neighbors are re-seeded to distance 1 every tick), so the horizon never rejects a
            // real belief; only a sourceless (dead-leader) belief, whose distance climbs without
            // bound as it circulates, ever crosses it — which is what lets a survivor take over.
            let heard = msg.leader.via_neighbor();
            if heard.is_valid() {
                best = best.merge(heard);
            }
        }
        // If no id we heard beats our own, WE are the leader: anchor at distance 0. Only a live
        // agent that is the local maximum ever (re)creates a distance-0 source.
        if best.leader <= self.id {
            best = LeaderBelief::own(self.id);
        }
        self.belief = best;

        // Stigmergy coverage decision: cover the cell if it is under the desired coverage level.
        // Purely local — depends only on THIS cell's mark count. Agents naturally spread effort
        // because well-covered cells stop attracting coverage.
        self.covering = local_level < COVERAGE_TARGET;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_agent_leads_itself() {
        let a = Agent::new(5, 1, 2);
        assert!(a.is_leader());
        assert_eq!(a.belief().leader, 5);
        assert_eq!(a.belief().distance, 0);
        assert_eq!(a.position(), (1, 2));
    }

    #[test]
    fn produce_addresses_every_neighbor() {
        let a = Agent::new(0, 0, 0);
        let msgs = a.produce(&[1, 2, 3]);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs.iter().map(|e| e.to).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert!(msgs.iter().all(|e| e.message.from == 0));
    }

    #[test]
    fn consume_adopts_a_higher_id_leader_from_a_neighbor() {
        let mut a = Agent::new(1, 0, 0);
        // Neighbor 9 claims itself as leader at distance 0.
        let inbox = vec![Message { from: 9, leader: LeaderBelief::own(9) }];
        a.consume(inbox, 0.0);
        assert_eq!(a.belief().leader, 9, "should defer to the higher-id leader");
        assert_eq!(a.belief().distance, 1, "one hop away via the neighbor");
        assert!(!a.is_leader());
    }

    #[test]
    fn consume_keeps_own_leadership_over_a_lower_id() {
        let mut a = Agent::new(7, 0, 0);
        let inbox = vec![Message { from: 2, leader: LeaderBelief::own(2) }];
        a.consume(inbox, 0.0);
        assert_eq!(a.belief().leader, 7, "higher own id wins");
        assert!(a.is_leader());
    }

    #[test]
    fn covering_follows_local_level_only() {
        let mut a = Agent::new(0, 0, 0);
        a.consume(vec![], 0.0);
        assert!(a.is_covering(), "under-covered cell -> cover");
        a.consume(vec![], COVERAGE_TARGET);
        assert!(!a.is_covering(), "sufficiently covered cell -> stop");
    }
}
