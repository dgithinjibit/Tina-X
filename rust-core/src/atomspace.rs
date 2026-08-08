//! Partitioned Atomspace: a tiny **hot working-space** the agent reasons over, kept separate from
//! large **cold** knowledge (implements ADR 0002).
//!
//! # Why this exists (the measured reason)
//!
//! Our P0.3 benchmark found direct `space.query()` over a flat Atomspace is ~O(n): ~2 s @100k
//! atoms, ~23 s @1M (`docs/benchmarks/metta-baseline.md`). P2.7 then measured the fix
//! (`metta-logic/bench_partition.py`): querying a SEPARATE tiny hot space stays **sub-millisecond
//! and flat** as cold knowledge grows (0.9 ms → 0.6 ms across 1k → 100k; flat `query()` grew 93×).
//!
//! So the rule (ADR 0002) is: **never scan a big flat space in a reasoning step.** Keep the
//! agent's current state in a small, bounded hot space; reach cold knowledge by keyed rules.
//!
//! # What this type does
//!
//! [`HotWorkingSpace`] is the Rust-side custodian of that hot partition. It holds a bounded set of
//! current-state facts as MeTTa atom text and renders them as a preamble the supervisor prepends
//! to a query. "Bounded" is the safety property: [`HotWorkingSpace::MAX_FACTS`] caps the hot set
//! so a reasoning query can never silently drift back into the O(n) regime. Cold knowledge stays
//! on disk in `metta-logic/knowledge/*.metta` and is reached by rule, never enumerated here.

use std::collections::VecDeque;

/// A bounded, in-memory hot working-space of current-state facts (as MeTTa atom text).
///
/// The agent asserts facts as it senses; the space keeps only the most recent
/// [`Self::MAX_FACTS`], evicting oldest-first. This bound is what keeps every query over the hot
/// space in the fast, size-independent regime measured in P2.7.
#[derive(Clone, Debug)]
pub struct HotWorkingSpace {
    /// Recent facts, newest at the back. A ring-like deque so eviction is O(1) and the working
    /// set can never grow unbounded (the whole point — see the O(n) risk in the module docs).
    facts: VecDeque<String>,
}

impl HotWorkingSpace {
    /// The hard cap on hot facts. Small by design: P2.7 showed even a handful keeps hot-space
    /// queries sub-millisecond regardless of cold-knowledge size. If a use case needs more, that
    /// is a signal to move data to cold knowledge + a keyed rule, not to raise this bound.
    pub const MAX_FACTS: usize = 32;

    /// A new, empty hot space.
    pub fn new() -> Self {
        Self { facts: VecDeque::with_capacity(Self::MAX_FACTS) }
    }

    /// Assert a fact into the hot space (as MeTTa atom text, e.g.
    /// `"(hot-state (rates 0.4 0 0.1) (mode free))"`). Oldest fact is evicted if at capacity, so
    /// the working set stays bounded and the O(n) risk cannot creep back in.
    pub fn assert_fact(&mut self, atom_text: impl Into<String>) {
        if self.facts.len() == Self::MAX_FACTS {
            self.facts.pop_front();
        }
        self.facts.push_back(atom_text.into());
    }

    /// How many facts are currently held.
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    /// True if the hot space holds no facts.
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Clear the hot space (e.g. on mission reset).
    pub fn clear(&mut self) {
        self.facts.clear();
    }

    /// Render the hot facts as a newline-separated MeTTa preamble the supervisor prepends to a
    /// query, so the reasoning sees current state WITHOUT any of it living in a large cold space.
    pub fn preamble(&self) -> String {
        let mut out = String::new();
        for f in &self.facts {
            out.push_str(f);
            out.push('\n');
        }
        out
    }
}

impl Default for HotWorkingSpace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asserts_and_renders_facts() {
        let mut hot = HotWorkingSpace::new();
        assert!(hot.is_empty());
        hot.assert_fact("(hot-state (rates 0.4 0 0.1) (mode free))");
        hot.assert_fact("(zone gusty)");
        assert_eq!(hot.len(), 2);
        let p = hot.preamble();
        assert!(p.contains("(hot-state (rates 0.4 0 0.1) (mode free))"));
        assert!(p.contains("(zone gusty)"));
        // Newline-separated, one per fact.
        assert_eq!(p.matches('\n').count(), 2);
    }

    #[test]
    fn stays_bounded_and_evicts_oldest() {
        // The safety property: the hot set never exceeds MAX_FACTS, so queries over it can't drift
        // into the O(n) regime no matter how many facts the agent asserts over a long mission.
        let mut hot = HotWorkingSpace::new();
        for i in 0..(HotWorkingSpace::MAX_FACTS + 10) {
            hot.assert_fact(format!("(tick {i})"));
        }
        assert_eq!(hot.len(), HotWorkingSpace::MAX_FACTS, "must cap at MAX_FACTS");
        let p = hot.preamble();
        // The 10 oldest were evicted; the newest is present, an early one is gone.
        assert!(p.contains(&format!("(tick {})", HotWorkingSpace::MAX_FACTS + 9)));
        assert!(!p.contains("(tick 0)"));
    }

    #[test]
    fn clear_empties_the_space() {
        let mut hot = HotWorkingSpace::new();
        hot.assert_fact("(a)");
        hot.clear();
        assert!(hot.is_empty());
        assert_eq!(hot.preamble(), "");
    }
}
