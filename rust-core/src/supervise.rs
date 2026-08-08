//! The **supervisory loop**: how the slow symbolic brain steers the fast reflex loop — safely.
//!
//! # The two-rate hand-off (read this first)
//!
//! Project Nzi has a fast reflex loop ([`crate::reflex`], <13 ms) and a slow symbolic brain
//! (MeTTa, milliseconds). This module is the SLOW side's control step. Each supervisory tick:
//!
//!   1. asks the brain to **propose** a setpoint by reasoning over the knowledge base
//!      (`metta-logic/knowledge/*.metta`) and the latest telemetry — `!(decide-setpoint …)`,
//!   2. runs that proposal through the **verification gate** ([`crate::verify::Gate`], Stage ①),
//!   3. and only forwards an **approved** setpoint down to the reflex loop.
//!
//! The gate sits *between* the two rates. A MeTTa call costs milliseconds, so — like the brain
//! and the gate — the supervisor belongs in the SLOW loop only and is **never** called from the
//! <13 ms reflex path. The reflex loop keeps holding the last approved setpoint between ticks.
//!
//! # Governance (fail-closed)
//!
//! Safety comes from what we do when we're *unsure*. The supervisor keeps the last **known-safe**
//! setpoint and holds it whenever a tick does not produce a fresh approved one — whether the gate
//! rejected the proposal, the brain returned nothing, or the answer was unparseable. The agent
//! refuses to act on doubt rather than acting confidently-but-wrong. See
//! `docs/adr/0006-supervisory-loop.md`.

use std::path::{Path, PathBuf};

use crate::brain::{MettaQuery, SubprocessBrain, SymbolicBrain};
use crate::reflex::Vec3;
use crate::telemetry::BrainDecision;
use crate::verify::{Context, GateError, Setpoint, Verdict};

/// The knowledge-base rule files, in dependency order (world defines helpers mission uses).
/// Same order as `metta-logic/run_supervise_smoke.py` — if they diverge, the smoke test lies.
pub const KB_LOAD_ORDER: [&str; 3] = ["world.metta", "mission.metta", "agent.metta"];

/// What the reflex loop should track right now, plus how we arrived at it. Returned every tick so
/// the caller (and the dashboard) can see both the ACTION and the JUSTIFICATION.
#[derive(Clone, Debug, PartialEq)]
pub struct SupervisorOutcome {
    /// The setpoint the reflex loop should hold — ALWAYS a known-safe value. On a fresh approval
    /// it's the new proposal; otherwise it's the retained last-safe setpoint (governance).
    pub setpoint: Vec3,
    /// Why: approved-and-updated, or held-because-<reason>.
    pub disposition: Disposition,
    /// A dashboard-ready record of this decision (query asked, atoms returned, verified flag).
    pub decision: BrainDecision,
}

/// The outcome of one supervisory tick, from a governance standpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// The brain proposed a setpoint and the gate approved it; the reflex target was updated.
    Approved,
    /// The gate rejected the proposal (names the failing check); we held the last-safe setpoint.
    Refused(String),
    /// The brain produced no usable proposal (empty/ambiguous/unparseable); held last-safe.
    /// Fail-closed: an unclear answer is treated as "do not act", never as approval.
    HeldOnDoubt(String),
}

impl Disposition {
    /// Did this tick hand a fresh, verified setpoint to the reflex loop?
    pub fn is_approved(&self) -> bool {
        matches!(self, Disposition::Approved)
    }
}

/// The current situation handed to the brain each tick. Kept small and typed so the caller can't
/// build a malformed telemetry term; the supervisor renders it to MeTTa.
#[derive(Clone, Debug, PartialEq)]
pub struct Telemetry {
    /// Latest measured body rates (rate gyros / halteres analogue).
    pub measured: Vec3,
    /// Whether the telemetry is recent enough to trust (feeds the gate's perception check).
    pub fresh: bool,
    /// Whether the measured values are finite (feeds the gate's perception check).
    pub finite: bool,
    /// Current mission mode, e.g. `hold-level` | `free`.
    pub mode: String,
}

impl Telemetry {
    /// Render as the MeTTa `(telemetry (measured (rates …)) (mode …))` term `decide-setpoint`
    /// destructures. Whole numbers render without a decimal (MeTTa normalizes `0.0` -> `0`).
    fn to_metta(&self) -> String {
        format!(
            "(telemetry (measured (rates {} {} {})) (mode {}))",
            fmt_num(self.measured.roll),
            fmt_num(self.measured.pitch),
            fmt_num(self.measured.yaw),
            self.mode,
        )
    }

    /// The verification [`Context`] for this telemetry. The supervisor stamps its own identity as
    /// the `source` (must be trusted by verification/communication.metta) so its OWN proposals
    /// aren't rejected as unattributed.
    fn to_context(&self, source: &str) -> Context {
        Context {
            measured: Setpoint::new(self.measured.roll, self.measured.pitch, self.measured.yaw),
            fresh: self.fresh,
            finite: self.finite,
            source: source.to_string(),
            mode: self.mode.clone(),
        }
    }
}

/// The slow supervisory controller. Holds a brain, the loaded KB preamble, the verification gate,
/// and the last-known-safe setpoint (governance state).
///
/// Generic over the brain so tests use `Supervisor<FakeBrain>` (no MeTTa) and production uses
/// `Supervisor<SubprocessBrain>`. Build with [`Supervisor::new`] or [`Supervisor::from_project_root`].
pub struct Supervisor<B: SymbolicBrain> {
    brain: B,
    /// Knowledge-base rules (world+mission+agent), loaded once and reused each tick.
    kb: String,
    /// The verification gate. The supervisor's gate and its brain share the SAME kind of brain so
    /// both talk to one MeTTa; we keep a second brain of the same type inside the gate.
    gate: crate::verify::Gate<B>,
    /// The identity we stamp on proposed actions (must be on the gate's trusted allow-list).
    source: String,
    /// The last setpoint the gate approved. Initialized to zero (a safe "hold still" default) and
    /// only ever replaced by another APPROVED setpoint — this is the heart of fail-closed governance.
    last_safe: Vec3,
}

impl<B: SymbolicBrain + Clone> Supervisor<B> {
    /// Build a supervisor from a brain, the KB preamble, and the verification preamble.
    ///
    /// The brain is cloned so the same transport backs both the KB query and the gate (the
    /// `FakeBrain`/`SubprocessBrain` both derive/impl `Clone` cheaply). `source` must be a value
    /// the verification allow-list trusts (e.g. `local-brain`).
    pub fn new(
        brain: B,
        kb: impl Into<String>,
        verification_preamble: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let gate = crate::verify::Gate::new(brain.clone(), verification_preamble);
        Self {
            brain,
            kb: kb.into(),
            gate,
            source: source.into(),
            last_safe: Vec3::ZERO,
        }
    }

    /// The setpoint the reflex loop is currently holding (last approved, or the zero default).
    pub fn last_safe(&self) -> Vec3 {
        self.last_safe
    }

    /// Run one supervisory tick. See the module docs for the full sequence. This performs TWO
    /// brain calls (propose, then verify) and so is a SLOW-loop operation — never call it from
    /// the reflex path.
    pub fn tick(&mut self, telemetry: &Telemetry, decision_id: u64) -> SupervisorOutcome {
        // 1. Ask the brain to propose a setpoint from the KB + telemetry.
        let query_text = format!("{}\n!(decide-setpoint {})", self.kb, telemetry.to_metta());
        let proposal_result = self.brain.query(&MettaQuery::new(query_text.clone()));

        let (proposal, results_for_record) = match proposal_result {
            Ok(r) => match parse_setpoint(&r.results) {
                Some(sp) => (sp, r.results),
                // Brain answered, but not with a parseable setpoint -> fail-closed hold.
                None => {
                    return self.hold_on_doubt(
                        decision_id,
                        query_text,
                        r.results,
                        "unparseable proposal".to_string(),
                    );
                }
            },
            // Brain call itself failed -> fail-closed hold.
            Err(e) => {
                return self.hold_on_doubt(
                    decision_id,
                    query_text,
                    Vec::new(),
                    format!("brain error: {e}"),
                );
            }
        };

        // 2. Verify the proposal through the gate, stamping our trusted identity as its source.
        let ctx = telemetry.to_context(&self.source);
        match self.gate.check(proposal, &ctx) {
            Ok(Verdict::Approved) => {
                // 3a. Approved: update the reflex target and the last-safe memory.
                let sp = Vec3::new(proposal.roll, proposal.pitch, proposal.yaw);
                self.last_safe = sp;
                SupervisorOutcome {
                    setpoint: sp,
                    disposition: Disposition::Approved,
                    decision: BrainDecision {
                        id: decision_id,
                        query: query_text,
                        results: results_for_record,
                        verified: true,
                    },
                }
            }
            Ok(Verdict::Rejected(check)) => {
                // 3b. Rejected: hold last-safe, surface which check refused it.
                SupervisorOutcome {
                    setpoint: self.last_safe,
                    disposition: Disposition::Refused(check.clone()),
                    decision: BrainDecision {
                        id: decision_id,
                        query: query_text,
                        results: vec![format!("(Rejected {check})")],
                        verified: false,
                    },
                }
            }
            // Gate error (rules unreadable / brain failed / unparseable verdict) -> fail-closed.
            Err(e) => self.hold_on_doubt(
                decision_id,
                query_text,
                results_for_record,
                format!("gate error: {}", gate_reason(&e)),
            ),
        }
    }

    /// Governance: retain the last-safe setpoint and record WHY we didn't take a fresh action.
    fn hold_on_doubt(
        &self,
        decision_id: u64,
        query: String,
        results: Vec<String>,
        reason: String,
    ) -> SupervisorOutcome {
        SupervisorOutcome {
            setpoint: self.last_safe,
            disposition: Disposition::HeldOnDoubt(reason),
            decision: BrainDecision { id: decision_id, query, results, verified: false },
        }
    }
}

impl Supervisor<SubprocessBrain> {
    /// A production supervisor wired to the real MeTTa subprocess, the on-disk knowledge base, and
    /// the on-disk verification rules, using the conventional project layout. Its `source` is
    /// `local-brain` (trusted by verification/communication.metta).
    pub fn from_project_root(root: impl AsRef<Path>) -> Result<Self, GateError> {
        let root = root.as_ref();
        let kb = load_knowledge(root)?;
        let verification = crate::verify::load_verification_preamble(root)?;
        let brain = SubprocessBrain::from_project_root(PathBuf::from(root));
        Ok(Self::new(brain, kb, verification, "local-brain"))
    }
}

/// Read and concatenate the knowledge-base files from `<root>/metta-logic/knowledge/`.
/// Like the verification loader, this keeps the `.metta` files the single source of truth.
pub fn load_knowledge(root: impl AsRef<Path>) -> Result<String, GateError> {
    let dir = root.as_ref().join("metta-logic").join("knowledge");
    let mut program = String::new();
    for name in KB_LOAD_ORDER {
        let path = dir.join(name);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| GateError::RulesUnavailable(format!("{}: {e}", path.display())))?;
        program.push_str(&text);
        program.push('\n');
    }
    Ok(program)
}

/// Parse `(setpoint (rates R P Y))` from the brain's result atoms into a typed [`Setpoint`].
///
/// Returns `None` (→ fail-closed hold) on anything that isn't exactly one well-formed setpoint —
/// empty results, multiple atoms, wrong shape, or non-numeric rates.
fn parse_setpoint(results: &[String]) -> Option<Setpoint> {
    let [atom] = results else { return None };
    // Strip the `(setpoint (rates ...))` wrappers and read the three numbers.
    let inner = atom
        .trim()
        .strip_prefix("(setpoint")?
        .trim()
        .strip_prefix("(rates")?
        .trim_end()
        .strip_suffix(')')? // closes (setpoint ...)
        .trim_end()
        .strip_suffix(')')?; // closes (rates ...)
    let nums: Vec<f32> = inner
        .split_whitespace()
        .map(|t| t.parse::<f32>())
        .collect::<Result<_, _>>()
        .ok()?;
    match nums.as_slice() {
        [r, p, y] => Some(Setpoint::new(*r, *p, *y)),
        _ => None,
    }
}

/// A short reason string for a gate error, for the decision record.
fn gate_reason(e: &GateError) -> String {
    match e {
        GateError::RulesUnavailable(m) => format!("rules: {m}"),
        GateError::Brain(b) => format!("brain: {b}"),
        GateError::Unparseable(m) => format!("verdict: {m}"),
    }
}

fn fmt_num(x: f32) -> String {
    if x.fract() == 0.0 && x.is_finite() {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::FakeBrain;

    // Minimal stand-in rules; the FakeBrain matches on the exact program text, so the actual rule
    // BODIES don't matter here — only that the supervisor builds the right query and reacts to the
    // right answer. Real KB/verification logic is exercised by the venv integration test.
    const KB: &str = "KB";
    const VERIF: &str = "VERIF";

    fn tlm(mode: &str) -> Telemetry {
        Telemetry {
            measured: Vec3::new(0.4, 0.0, 0.1),
            fresh: true,
            finite: true,
            mode: mode.to_string(),
        }
    }

    // The exact query the supervisor will build for a given telemetry — used to program the fake.
    fn decide_query(t: &Telemetry) -> String {
        format!("{KB}\n!(decide-setpoint {})", t.to_metta())
    }

    // The exact gate program for a proposal+telemetry — Gate<FakeBrain> builds this internally.
    fn gate_query(proposal: &str, t: &Telemetry) -> String {
        format!(
            "{VERIF}\n!(gate-setpoint {} {})",
            proposal,
            t.to_context("local-brain").to_metta()
        )
    }

    fn supervisor(brain: FakeBrain) -> Supervisor<FakeBrain> {
        Supervisor::new(brain, KB, VERIF, "local-brain")
    }

    #[test]
    fn approved_proposal_updates_last_safe_and_forwards() {
        let t = tlm("free");
        let proposal = "(setpoint (rates 0.5 0 0.2))";
        let brain = FakeBrain::new()
            .with_answer(decide_query(&t), vec![proposal.to_string()])
            .with_answer(gate_query(proposal, &t), vec!["Approved".to_string()]);
        let mut sup = supervisor(brain);

        let out = sup.tick(&t, 1);
        assert_eq!(out.disposition, Disposition::Approved);
        assert_eq!(out.setpoint, Vec3::new(0.5, 0.0, 0.2));
        assert!(out.decision.verified);
        assert_eq!(sup.last_safe(), Vec3::new(0.5, 0.0, 0.2));
    }

    #[test]
    fn rejected_proposal_holds_last_safe() {
        // First approve something so last_safe is non-zero, then get a rejection and confirm hold.
        let t = tlm("free");
        let good = "(setpoint (rates 0.5 0 0.2))";
        let bad = "(setpoint (rates 9 0 0.2))"; // over-envelope; gate will reject on reasoning
        let brain = FakeBrain::new()
            .with_answer(decide_query(&t), vec![good.to_string()])
            .with_answer(gate_query(good, &t), vec!["Approved".to_string()]);
        let mut sup = supervisor(brain);
        sup.tick(&t, 1); // last_safe now (0.5,0,0.2)

        // Now swap the brain to propose the bad setpoint that the gate rejects.
        let brain2 = FakeBrain::new()
            .with_answer(decide_query(&t), vec![bad.to_string()])
            .with_answer(gate_query(bad, &t), vec!["(Rejected reasoning)".to_string()]);
        sup.brain = brain2.clone();
        sup.gate = crate::verify::Gate::new(brain2, VERIF);

        let out = sup.tick(&t, 2);
        assert_eq!(out.disposition, Disposition::Refused("reasoning".to_string()));
        assert_eq!(out.setpoint, Vec3::new(0.5, 0.0, 0.2), "must hold the last-safe setpoint");
        assert!(!out.decision.verified);
    }

    #[test]
    fn empty_brain_answer_fails_closed() {
        // Brain returns nothing for the decide query -> HeldOnDoubt, setpoint stays the zero default.
        let t = tlm("free");
        let mut sup = supervisor(FakeBrain::new());
        let out = sup.tick(&t, 1);
        assert!(matches!(out.disposition, Disposition::HeldOnDoubt(_)));
        assert_eq!(out.setpoint, Vec3::ZERO);
        assert!(!out.decision.verified);
    }

    #[test]
    fn parse_setpoint_reads_well_formed_and_rejects_garbage() {
        assert_eq!(
            parse_setpoint(&["(setpoint (rates 0.5 0 0.2))".to_string()]),
            Some(Setpoint::new(0.5, 0.0, 0.2))
        );
        assert_eq!(parse_setpoint(&[]), None);
        assert_eq!(parse_setpoint(&["nonsense".to_string()]), None);
        assert_eq!(
            parse_setpoint(&["(setpoint (rates 0.5 0))".to_string()]),
            None,
            "wrong arity must fail closed"
        );
    }
}
