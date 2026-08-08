//! SoNS hierarchy formation: decentralized leader ("brain") election (Phase 4, P4.2).
//!
//! # The idea
//! A **Self-organizing Nervous System** forms a hierarchy at runtime from neighbor-local gossip,
//! with a single transient "brain" (leader) per connected group that is **interchangeable** — if
//! it fails, the group re-elects another with no central coordinator.
//!
//! We implement this as a distributed **max-consensus with distance**: every agent gossips its
//! current [`LeaderBelief`]; each tick an agent adopts the "best" belief it has seen. The ordering
//! makes the **highest agent id** the leader (a stable, deterministic choice needing no shared
//! clock or ids-registry), and tracks hop-distance to it so agents also learn a gradient TOWARD
//! the brain — the nervous-system backbone.
//!
//! # Why this self-heals (the whole point)
//! Belief in a leader is only *sustained* by fresh gossip carrying that leader id. When the leader
//! dies it stops gossiping; the id no longer enters the network; and because every agent re-seeds
//! "I could lead myself" each tick ([`LeaderBelief::own`]), the surviving highest id wins the next
//! rounds of consensus. The brain is replaced automatically — no failure detector, no election
//! message, just the same rule continuing to run. (Convergence takes O(diameter) ticks.)

/// Maximum hops a leader belief may travel from its source before it is considered stale and
/// rejected. Must exceed the largest graph diameter the swarm runs on (demos/tests top out at a
/// 20×20 grid, diameter ~38) so it NEVER rejects a belief from a live leader; it exists only to
/// bound the classic "count to infinity" of a sourceless belief.
///
/// Recovery cost: after a leader dies, its stale belief takes ~`MAX_HORIZON` ticks to drain before
/// a survivor is re-elected — recovery is O(horizon), a deliberate, bounded, deterministic cost
/// (the same technique distance-vector routing protocols use). Keep it just above the largest
/// diameter so recovery is as quick as the count-to-infinity bound allows.
pub const MAX_HORIZON: u32 = 40;

/// One agent's belief about who leads its connected component, and how far away that leader is.
///
/// Ordered so [`LeaderBelief::merge`] is a clean max: a HIGHER `leader` id always wins; for the
/// same leader, the SHORTER `distance` wins (the fresher/closer path to the brain).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaderBelief {
    /// The id of the believed leader ("brain") of this agent's component.
    pub leader: u32,
    /// Hop distance from this agent to that leader along the gossip path. 0 means "I am it".
    pub distance: u32,
}

impl LeaderBelief {
    /// The seed belief "I lead myself, distance 0". Re-seeded every tick so a dead leader's id can
    /// drain out of the network and a survivor can take over.
    pub fn own(id: u32) -> Self {
        Self { leader: id, distance: 0 }
    }

    /// This belief as heard THROUGH a neighbor: same leader, one more hop away. Distance saturates
    /// so it can never overflow on a pathological topology.
    pub fn via_neighbor(self) -> Self {
        Self { leader: self.leader, distance: self.distance.saturating_add(1) }
    }

    /// Is this belief still fresh enough to be trusted? A belief is only VALID within
    /// [`MAX_HORIZON`] hops of its source. This is the mechanism that lets a dead leader's belief
    /// expire: with the real source gone, agents keep gossiping the id in a cycle and its distance
    /// climbs every tick; once it exceeds the horizon the belief is rejected and a survivor
    /// re-elects itself. The horizon is set comfortably above any real graph diameter so it never
    /// interferes with legitimate propagation from a LIVE leader (whose neighbors re-seed distance
    /// 1 every tick, keeping distances low).
    pub fn is_valid(self) -> bool {
        self.distance <= MAX_HORIZON
    }

    /// Combine two beliefs, keeping the "better" one under the SoNS ordering:
    ///   * strictly higher leader id wins (that agent is the elected brain);
    ///   * same leader -> the shorter distance wins (closer/fresher path).
    pub fn merge(self, other: Self) -> Self {
        // `other` wins iff it names a higher leader, or the same leader by a shorter path.
        let other_wins = other.leader > self.leader
            || (other.leader == self.leader && other.distance < self.distance);
        if other_wins {
            other
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_leader_id_wins() {
        let a = LeaderBelief { leader: 3, distance: 0 };
        let b = LeaderBelief { leader: 8, distance: 5 };
        assert_eq!(a.merge(b).leader, 8);
        assert_eq!(b.merge(a).leader, 8, "merge is symmetric in the winner");
    }

    #[test]
    fn same_leader_prefers_shorter_distance() {
        let near = LeaderBelief { leader: 5, distance: 2 };
        let far = LeaderBelief { leader: 5, distance: 9 };
        assert_eq!(near.merge(far).distance, 2);
        assert_eq!(far.merge(near).distance, 2);
    }

    #[test]
    fn via_neighbor_adds_one_hop_and_saturates() {
        let b = LeaderBelief { leader: 5, distance: 2 };
        assert_eq!(b.via_neighbor().distance, 3);
        let max = LeaderBelief { leader: 5, distance: u32::MAX };
        assert_eq!(max.via_neighbor().distance, u32::MAX, "must not overflow");
    }

    #[test]
    fn horizon_rejects_only_stale_beliefs() {
        // A belief within the horizon is valid; one past it (a circulating dead-leader belief) is
        // rejected, which is what lets a survivor be re-elected.
        assert!(LeaderBelief { leader: 9, distance: MAX_HORIZON }.is_valid());
        assert!(!LeaderBelief { leader: 9, distance: MAX_HORIZON + 1 }.is_valid());
    }
}
