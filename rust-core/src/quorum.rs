//! The **quorum arbiter**: the Rust side of the honeybee collective-decision (bee brain B1–B4).
//!
//! # What this is
//! Phase-4 SoNS ([`nzi_swarm`], a separate crate) elects ONE transient "brain" per connected group
//! via cheap neighbor-local max-consensus. That answers *who coordinates*. This module answers the
//! next question — *what does the group commit to* — for choices where committing on weak evidence
//! is dangerous (which field to service, which rally-point, which of several near-equal setpoints).
//!
//! It is the Rust bridge to `metta-logic/knowledge/quorum.metta`. The cheap per-agent signals
//! (support tallies, stop-signal emission) are gathered in fast gossip; this arbiter turns those
//! tallies into a *justified* commit/scout verdict by asking the SLOW MeTTa layer:
//!
//!   1. the caller supplies neighbor-local [`Tally`]s (one option, its support, stop-signals recv'd)
//!      and a [`Risk`] level (the speed/accuracy knob — B2),
//!   2. we render them to the FLAT positional MeTTa data `quorum.metta` expects and query
//!      `!(decide-quorum (qctx (risk …)) <first> <rest-list>)`,
//!   3. we parse the reduced verdict into a typed [`QuorumVerdict`].
//!
//! # Why it lives in the SLOW loop
//! Like the [`crate::supervise`] tick, this performs a MeTTa call (milliseconds) and is **never**
//! run from the <13 ms reflex path. Cross-inhibition (B3) and the risk-tuned threshold (B2/B4) are
//! deliberation, not reflex.
//!
//! # MeTTa gotcha (mirrors `run_quorum_smoke.py`)
//! Options MUST be written FLAT — `(opt north 7 0)` — never with `(id ..)`/`(support ..)`
//! sub-wrappers, which MeTTa would evaluate away before the accessors match (see
//! `nzi-metta-gotchas`). [`Tally::to_metta`] enforces the flat form.

use std::path::Path;

use crate::brain::{BrainError, MettaQuery, SymbolicBrain};

/// The risk level of the decision — the single quorum tuning knob (bee brain B2).
///
/// `High` demands strong agreement before committing (irreversible / high-stakes actions) and turns
/// cross-inhibition OFF so it can't trap a high-quorum decision in a local optimum (B4). `Low`
/// commits sooner (time-critical / reversible) and turns cross-inhibition ON to break ties (B3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    High,
    Low,
}

impl Risk {
    /// The MeTTa symbol used inside `(qctx (risk …))`.
    fn as_symbol(self) -> &'static str {
        match self {
            Risk::High => "high",
            Risk::Low => "low",
        }
    }
}

/// One option's neighbor-local tally, as seen by the deciding ("brain") agent.
///
/// `support` is how many scouts back this option; `inhibition` is the count of stop-signals it
/// received from scouts backing rival options (the honeybee cross-inhibition signal). Both are
/// plain counts built in fast Rust gossip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tally {
    /// The option's identifier (a bare MeTTa symbol, e.g. `north`, `field-3`). Kept simple: it is
    /// emitted verbatim into the flat option term, so it must be a single atom with no spaces.
    pub id: String,
    /// How many neighbor scouts currently back this option.
    pub support: u32,
    /// Stop-signals received from scouts backing rivals (cross-inhibition, B3).
    pub inhibition: u32,
}

impl Tally {
    /// Convenience constructor.
    pub fn new(id: impl Into<String>, support: u32, inhibition: u32) -> Self {
        Self { id: id.into(), support, inhibition }
    }

    /// Render as the FLAT positional MeTTa term `(opt $id $support $inhibition)` the accessors in
    /// `quorum.metta` destructure. FLAT is mandatory (see the module gotcha).
    fn to_metta(&self) -> String {
        format!("(opt {} {} {})", self.id, self.support, self.inhibition)
    }
}

/// The typed result of a quorum decision — the parsed form of `quorum.metta`'s verdict atom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuorumVerdict {
    /// The best-supported option crossed the risk-tuned quorum: commit to it with this effective
    /// weight. Mirrors `(Commit $id $weight quorum-reached)`.
    Commit { id: String, weight: u32 },
    /// No option has crossed quorum yet — keep scouting (fail-safe default). Mirrors
    /// `(Scout below-quorum)`.
    Scout,
}

/// Everything that can go wrong turning tallies into a verdict.
#[derive(Debug)]
pub enum QuorumError {
    /// No options were supplied — there is nothing to decide over.
    NoOptions,
    /// The MeTTa rule file couldn't be read.
    RulesUnavailable(String),
    /// The brain call itself failed.
    Brain(BrainError),
    /// The brain answered, but not with a verdict we could parse (contract violation / unreduced).
    Unparseable(String),
}

impl std::fmt::Display for QuorumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuorumError::NoOptions => write!(f, "no options to decide over"),
            QuorumError::RulesUnavailable(m) => write!(f, "quorum rules unavailable: {m}"),
            QuorumError::Brain(e) => write!(f, "brain error: {e}"),
            QuorumError::Unparseable(m) => write!(f, "unparseable quorum verdict: {m}"),
        }
    }
}

impl std::error::Error for QuorumError {}

/// The slow quorum arbiter. Holds a brain and the loaded `quorum.metta` preamble.
///
/// Generic over the brain so tests use `QuorumArbiter<FakeBrain>` (no MeTTa) and production uses
/// `QuorumArbiter<SubprocessBrain>`. Build with [`QuorumArbiter::new`] or
/// [`QuorumArbiter::from_project_root`].
pub struct QuorumArbiter<B: SymbolicBrain> {
    brain: B,
    /// The `quorum.metta` rules, loaded once and reused each decision.
    rules: String,
}

impl<B: SymbolicBrain> QuorumArbiter<B> {
    /// Build an arbiter from a brain and the `quorum.metta` rule text.
    pub fn new(brain: B, rules: impl Into<String>) -> Self {
        Self { brain, rules: rules.into() }
    }

    /// Decide over neighbor-local `tallies` under a `risk` level.
    ///
    /// This is the Rust analog of one `decide-quorum` call in `run_quorum_smoke.py`: it builds the
    /// context, the flat first option, and the cons-list of the rest, then parses the verdict.
    /// Returns [`QuorumError::NoOptions`] if `tallies` is empty (fail-safe: nothing to commit to).
    pub fn decide(&self, risk: Risk, tallies: &[Tally]) -> Result<QuorumVerdict, QuorumError> {
        let (first, rest) = tallies.split_first().ok_or(QuorumError::NoOptions)?;

        let ctx = format!("(qctx (risk {}))", risk.as_symbol());
        let query_text = format!(
            "{}\n!(decide-quorum {} {} {})",
            self.rules,
            ctx,
            first.to_metta(),
            cons_list(rest),
        );

        let result = self
            .brain
            .query(&MettaQuery::new(query_text))
            .map_err(QuorumError::Brain)?;

        parse_verdict(&result.results)
    }
}

impl QuorumArbiter<crate::brain::SubprocessBrain> {
    /// A production arbiter wired to the real MeTTa subprocess and the on-disk `quorum.metta`,
    /// using the conventional project layout (`<root>/metta-logic/knowledge/quorum.metta`).
    pub fn from_project_root(root: impl AsRef<Path>) -> Result<Self, QuorumError> {
        let root = root.as_ref();
        let rules = load_quorum_rules(root)?;
        let brain = crate::brain::SubprocessBrain::from_project_root(root.to_path_buf());
        Ok(Self::new(brain, rules))
    }
}

/// Read `<root>/metta-logic/knowledge/quorum.metta`, the single source of truth for the rules.
pub fn load_quorum_rules(root: impl AsRef<Path>) -> Result<String, QuorumError> {
    let path = root
        .as_ref()
        .join("metta-logic")
        .join("knowledge")
        .join("quorum.metta");
    std::fs::read_to_string(&path)
        .map_err(|e| QuorumError::RulesUnavailable(format!("{}: {e}", path.display())))
}

/// Build a MeTTa cons-list `(:: o1 (:: o2 … ()))` from option terms, exactly as
/// `run_quorum_smoke.py`'s `cons_list` does. An empty slice yields `()`.
fn cons_list(items: &[Tally]) -> String {
    let mut out = String::from("()");
    for it in items.iter().rev() {
        out = format!("(:: {} {})", it.to_metta(), out);
    }
    out
}

/// Parse the reduced verdict atom into a typed [`QuorumVerdict`].
///
/// Accepts EXACTLY one of `(Commit $id $weight quorum-reached)` or `(Scout below-quorum)`.
/// Anything else — empty results, multiple atoms, an unreduced `(if …)` blob, a non-numeric weight
/// — is a contract violation and returns [`QuorumError::Unparseable`] (fail-safe: the caller must
/// NOT treat an unclear answer as a commit).
fn parse_verdict(results: &[String]) -> Result<QuorumVerdict, QuorumError> {
    let [atom] = results else {
        return Err(QuorumError::Unparseable(format!(
            "expected exactly one verdict atom, got {results:?}"
        )));
    };
    let trimmed = atom.trim();

    if trimmed == "(Scout below-quorum)" {
        return Ok(QuorumVerdict::Scout);
    }

    // (Commit <id> <weight> quorum-reached)
    if let Some(inner) = trimmed
        .strip_prefix("(Commit")
        .and_then(|s| s.strip_suffix(')'))
    {
        let toks: Vec<&str> = inner.split_whitespace().collect();
        if let [id, weight, "quorum-reached"] = toks.as_slice() {
            let weight = weight
                .parse::<u32>()
                .map_err(|_| QuorumError::Unparseable(format!("non-numeric weight in {trimmed:?}")))?;
            return Ok(QuorumVerdict::Commit { id: (*id).to_string(), weight });
        }
    }

    Err(QuorumError::Unparseable(format!("not a quorum verdict: {trimmed:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::FakeBrain;

    // The exact query the arbiter builds, so we can program the fake brain to answer it. Mirrors
    // decide()'s construction (rules preamble "Q" here since FakeBrain matches on full text).
    fn query_for(rules: &str, risk: Risk, tallies: &[Tally]) -> String {
        let (first, rest) = tallies.split_first().unwrap();
        format!(
            "{}\n!(decide-quorum (qctx (risk {})) {} {})",
            rules,
            risk.as_symbol(),
            first.to_metta(),
            cons_list(rest),
        )
    }

    const RULES: &str = "Q";

    #[test]
    fn to_metta_is_flat_positional() {
        // The flat form is load-bearing (MeTTa gotcha): no (id ..)/(support ..) sub-wrappers.
        assert_eq!(Tally::new("north", 7, 0).to_metta(), "(opt north 7 0)");
    }

    #[test]
    fn cons_list_matches_the_smoke_test_shape() {
        assert_eq!(cons_list(&[]), "()");
        assert_eq!(
            cons_list(&[Tally::new("south", 8, 5)]),
            "(:: (opt south 8 5) ())"
        );
        assert_eq!(
            cons_list(&[Tally::new("a", 1, 0), Tally::new("b", 2, 0)]),
            "(:: (opt a 1 0) (:: (opt b 2 0) ()))"
        );
    }

    #[test]
    fn parses_a_commit_verdict() {
        assert_eq!(
            parse_verdict(&["(Commit north 7 quorum-reached)".to_string()]).unwrap(),
            QuorumVerdict::Commit { id: "north".to_string(), weight: 7 }
        );
    }

    #[test]
    fn parses_a_scout_verdict() {
        assert_eq!(
            parse_verdict(&["(Scout below-quorum)".to_string()]).unwrap(),
            QuorumVerdict::Scout
        );
    }

    #[test]
    fn rejects_unreduced_or_garbage_verdicts() {
        // Empty, multiple atoms, an unreduced (if ...) blob, and a bad weight all fail closed.
        assert!(parse_verdict(&[]).is_err());
        assert!(parse_verdict(&["(Scout below-quorum)".into(), "(extra)".into()]).is_err());
        assert!(parse_verdict(&["(if (>= 3 6) foo bar)".to_string()]).is_err());
        assert!(parse_verdict(&["(Commit north x quorum-reached)".to_string()]).is_err());
    }

    #[test]
    fn no_options_is_an_error_not_a_commit() {
        let arbiter = QuorumArbiter::new(FakeBrain::new(), RULES);
        assert!(matches!(arbiter.decide(Risk::Low, &[]), Err(QuorumError::NoOptions)));
    }

    #[test]
    fn decide_commits_when_the_brain_says_so() {
        // Low-risk, lone option support 7 -> the smoke test's "low-commit" case.
        let tallies = [Tally::new("north", 7, 0)];
        let brain = FakeBrain::new().with_answer(
            query_for(RULES, Risk::Low, &tallies),
            vec!["(Commit north 7 quorum-reached)".to_string()],
        );
        let arbiter = QuorumArbiter::new(brain, RULES);
        assert_eq!(
            arbiter.decide(Risk::Low, &tallies).unwrap(),
            QuorumVerdict::Commit { id: "north".to_string(), weight: 7 }
        );
    }

    #[test]
    fn decide_scouts_below_quorum() {
        // Low-risk, support 5 < quorum 6 -> Scout (the "low-below-quorum" case).
        let tallies = [Tally::new("north", 5, 0)];
        let brain = FakeBrain::new().with_answer(
            query_for(RULES, Risk::Low, &tallies),
            vec!["(Scout below-quorum)".to_string()],
        );
        let arbiter = QuorumArbiter::new(brain, RULES);
        assert_eq!(arbiter.decide(Risk::Low, &tallies).unwrap(), QuorumVerdict::Scout);
    }

    #[test]
    fn decide_surfaces_an_unparseable_answer_as_error() {
        // Brain answers, but not with a verdict -> fail-closed error (never a silent commit).
        let tallies = [Tally::new("north", 7, 0)];
        let brain = FakeBrain::new().with_answer(
            query_for(RULES, Risk::Low, &tallies),
            vec!["(if (>= 7 6) commit scout)".to_string()],
        );
        let arbiter = QuorumArbiter::new(brain, RULES);
        assert!(matches!(
            arbiter.decide(Risk::Low, &tallies),
            Err(QuorumError::Unparseable(_))
        ));
    }
}
