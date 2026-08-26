//! The **action-gate**: Project TINA-X's verification moat, in Rust (P2.2).
//!
//! # Why this module exists (read this first if you're new)
//!
//! The slow symbolic brain proposes an action — a *setpoint* (the body-rate it wants the reflex
//! loop to hold). Before that action is allowed to reach the reflex/actuator layer, it is run
//! through symbolic checks that catch the five well-known **agent-hallucination types**
//! (Perception, Communication, Reasoning, Execution, Memorization). An action that fails any
//! check is *rejected with a reason* — the agent refuses to act rather than acting
//! confidently-but-wrong. This refusal-under-doubt is the product's defensibility.
//!
//! # The split (safety-critical stays in Rust)
//!
//! The *reasoning rules* live in `metta-logic/verification/*.metta` (one file per hallucination
//! type — see that folder's README). This module is the **Rust gate** that:
//!   1. loads those rule files (single source of truth on disk — see [`load_verification_preamble`]),
//!   2. builds a MeTTa program: the rules + one `!(gate-setpoint <action> <context>)` line,
//!   3. runs it through the [`SymbolicBrain`] seam (subprocess to MeTTa; a fake in tests), and
//!   4. parses the answer into a [`Verdict`] — `Approved` or `Rejected(check-name)`.
//!
//! Only *approved* actions should ever be handed to the reflex layer. The gate decision itself
//! (approve/deny) is made in Rust; only the *reasoning* is delegated to MeTTa. This mirrors
//! `docs/adr/0003-rust-metta-bridge.md`: MeTTa is Python-bound and lives in the slow loop, but
//! the safety-critical yes/no stays in compiled Rust.
//!
//! # IMPORTANT rule
//! The gate calls the brain, which costs milliseconds. Like the brain itself, it belongs in the
//! SLOW supervisory loop only — **never inside the <13 ms reflex path**. It sits *between* the
//! brain and the reflex loop, vetting setpoints before they cross over.

use std::path::{Path, PathBuf};

use crate::brain::{BrainError, MettaQuery, SymbolicBrain};

/// The verification rule files, in the exact dependency order the gate concatenates them.
///
/// `limits.metta` defines shared constants/helpers the checks use; the five check files define
/// `check-<type>?`; `verify.metta` defines `gate-setpoint` which *calls* those checks, so it must
/// come last. This is the SAME order as `metta-logic/run_verify_smoke.py`'s `LOAD_ORDER` — if the
/// two ever disagree, the smoke test is lying about what the gate runs.
pub const LOAD_ORDER: [&str; 7] = [
    "limits.metta",
    "perception.metta",
    "reasoning.metta",
    "execution.metta",
    "memorization.metta",
    "communication.metta",
    "verify.metta",
];

/// A proposed action from the slow brain: the desired body-rates (rad/s) for the reflex loop.
///
/// This is the only action type the Phase-2 gate verifies (see the verification README). It is a
/// typed struct rather than raw MeTTa text so callers can't accidentally build a malformed action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setpoint {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Setpoint {
    pub fn new(roll: f32, pitch: f32, yaw: f32) -> Self {
        Self { roll, pitch, yaw }
    }

    /// Render as the MeTTa action term the checks match on: `(setpoint (rates R P Y))`.
    fn to_metta(self) -> String {
        format!(
            "(setpoint (rates {} {} {}))",
            fmt_num(self.roll),
            fmt_num(self.pitch),
            fmt_num(self.yaw)
        )
    }
}

/// The situation the brain based its decision on — everything the checks need to judge the action.
///
/// Field meanings mirror the `(context ...)` term in the verification README:
/// - `measured`: the latest gyro reading (used by the execution/step check),
/// - `fresh`: is the telemetry recent enough to trust? (perception check),
/// - `finite`: are the sensor values finite — no NaN/inf from a broken gyro? (perception check),
/// - `source`: who issued this action, for the communication allow-list,
/// - `mode`: current mission mode, e.g. `hold-level` | `free` (memorization invariant).
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub measured: Setpoint,
    pub fresh: bool,
    pub finite: bool,
    pub source: String,
    pub mode: String,
}

impl Context {
    /// Render as the MeTTa `(context ...)` term. Field order must match the `.metta` accessors,
    /// which destructure positionally: `(context $measured $fresh $finite $source $mode)`.
    ///
    /// Crate-visible so the supervisory-loop tests can build the exact gate query the gate emits
    /// internally (they program a `FakeBrain` that matches on that text).
    pub(crate) fn to_metta(&self) -> String {
        format!(
            "(context (measured (rates {} {} {})) {} {} {} {})",
            fmt_num(self.measured.roll),
            fmt_num(self.measured.pitch),
            fmt_num(self.measured.yaw),
            fmt_bool(self.fresh),
            fmt_bool(self.finite),
            self.source,
            self.mode,
        )
    }
}

/// The gate's decision. This is the whole point of the module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every check passed — this action is safe to hand to the reflex layer.
    Approved,
    /// A check failed; the string names WHICH one (e.g. `"perception"`), so the dashboard and
    /// logs can show the agent *why* it refused. Matches MeTTa's `(Rejected <check>)`.
    Rejected(String),
}

impl Verdict {
    /// The one question callers usually ask: may this action proceed? Maps directly onto the
    /// `verified: bool` field of `telemetry::BrainDecision`.
    pub fn is_approved(&self) -> bool {
        matches!(self, Verdict::Approved)
    }
}

/// Everything that can go wrong *gating* an action (distinct from talking to the brain).
#[derive(Debug)]
pub enum GateError {
    /// A verification `.metta` rule file couldn't be read from disk.
    RulesUnavailable(String),
    /// The underlying brain call failed (spawn/worker/protocol) — see [`BrainError`].
    Brain(BrainError),
    /// MeTTa returned something that isn't `Approved` or `(Rejected ...)` — a contract violation
    /// that we surface loudly rather than silently treating as approve/deny.
    Unparseable(String),
}

impl std::fmt::Display for GateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateError::RulesUnavailable(m) => write!(f, "cannot load verification rules: {m}"),
            GateError::Brain(e) => write!(f, "brain call failed during verification: {e}"),
            GateError::Unparseable(m) => write!(f, "verification returned unparseable verdict: {m}"),
        }
    }
}

impl std::error::Error for GateError {}

impl From<BrainError> for GateError {
    fn from(e: BrainError) -> Self {
        GateError::Brain(e)
    }
}

/// Read and concatenate the verification rule files from `<root>/metta-logic/verification/`.
///
/// This is what makes the `.metta` files the single source of truth: edit a rule there and the
/// gate picks it up on the next call — no Rust rebuild. Returns the whole preamble as one string,
/// with the files joined by newlines in [`LOAD_ORDER`].
pub fn load_verification_preamble(root: impl AsRef<Path>) -> Result<String, GateError> {
    let dir = root.as_ref().join("metta-logic").join("verification");
    let mut program = String::new();
    for name in LOAD_ORDER {
        let path = dir.join(name);
        let text = std::fs::read_to_string(&path).map_err(|e| {
            GateError::RulesUnavailable(format!("{}: {e}", path.display()))
        })?;
        program.push_str(&text);
        program.push('\n');
    }
    Ok(program)
}

/// The action-gate: wraps any [`SymbolicBrain`] and vets setpoints against the MeTTa checks.
///
/// Generic over the brain so tests use `Gate<FakeBrain>` (no MeTTa needed, microsecond-fast) and
/// production uses `Gate<SubprocessBrain>` (the real reasoning). Construct with [`Gate::new`] and
/// give it the preamble once; [`Gate::check`] then verifies as many actions as you like.
pub struct Gate<B: SymbolicBrain> {
    brain: B,
    /// The verification rules, loaded once and reused for every `check` (they don't change between
    /// calls within a run). Held as text because that's exactly what the brain seam consumes.
    preamble: String,
}

impl<B: SymbolicBrain> Gate<B> {
    /// Build a gate from a brain and an already-loaded rule preamble (see
    /// [`load_verification_preamble`]). Prefer [`Gate::from_project_root`] for the common case.
    pub fn new(brain: B, preamble: impl Into<String>) -> Self {
        Self { brain, preamble: preamble.into() }
    }

    /// Verify a proposed `action` against its `context`. Returns the [`Verdict`].
    ///
    /// Builds `<preamble>\n!(gate-setpoint <action> <context>)`, runs it through the brain, and
    /// interprets the single result atom. Only [`Verdict::Approved`] means "safe to act".
    pub fn check(&self, action: Setpoint, ctx: &Context) -> Result<Verdict, GateError> {
        let program = format!(
            "{}\n!(gate-setpoint {} {})",
            self.preamble,
            action.to_metta(),
            ctx.to_metta()
        );
        let result = self.brain.query(&MettaQuery::new(program))?;
        parse_verdict(&result.results)
    }
}

impl Gate<crate::brain::SubprocessBrain> {
    /// Convenience: a production gate wired to the real MeTTa subprocess and the on-disk rules,
    /// using the conventional project layout. Fails if the rules can't be read.
    pub fn from_project_root(root: impl AsRef<Path>) -> Result<Self, GateError> {
        let root = root.as_ref();
        let preamble = load_verification_preamble(root)?;
        let brain = crate::brain::SubprocessBrain::from_project_root(PathBuf::from(root));
        Ok(Self::new(brain, preamble))
    }
}

/// Turn MeTTa's result atoms into a [`Verdict`].
///
/// The gate composer (`verify.metta`) is written to return EXACTLY one atom: `Approved` or
/// `(Rejected <check>)`. We accept the single expected atom and reject anything else loudly —
/// a fail-closed posture (an unrecognized answer is NOT treated as approval).
fn parse_verdict(results: &[String]) -> Result<Verdict, GateError> {
    let [atom] = results else {
        return Err(GateError::Unparseable(format!(
            "expected exactly one result atom, got {results:?}"
        )));
    };
    let atom = atom.trim();
    if atom == "Approved" {
        return Ok(Verdict::Approved);
    }
    // Expect `(Rejected <check>)`. Strip the wrapper and take the check name.
    if let Some(inner) = atom.strip_prefix("(Rejected ").and_then(|s| s.strip_suffix(')')) {
        let check = inner.trim();
        if !check.is_empty() {
            return Ok(Verdict::Rejected(check.to_string()));
        }
    }
    Err(GateError::Unparseable(atom.to_string()))
}

/// Format a float for MeTTa without a trailing `.0`-less integer surprise. MeTTa parses `3` and
/// `3.0` differently in some contexts; the checks compare against literals like `0`, so we render
/// whole numbers as bare integers and everything else with its decimal.
fn fmt_num(x: f32) -> String {
    if x.fract() == 0.0 && x.is_finite() {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

fn fmt_bool(b: bool) -> String {
    if b { "True".to_string() } else { "False".to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::FakeBrain;

    // A base context that passes every check (mirrors run_verify_smoke.py's GOOD_CTX).
    fn good_ctx() -> Context {
        Context {
            measured: Setpoint::new(0.4, 0.0, 0.1),
            fresh: true,
            finite: true,
            source: "local-brain".to_string(),
            mode: "free".to_string(),
        }
    }

    #[test]
    fn setpoint_renders_whole_numbers_without_decimal() {
        // hold-level memorization compares roll/pitch against literal `0`, so 0.0 must render `0`.
        assert_eq!(Setpoint::new(0.0, 0.0, 0.0).to_metta(), "(setpoint (rates 0 0 0))");
        assert_eq!(Setpoint::new(0.5, 0.0, 0.2).to_metta(), "(setpoint (rates 0.5 0 0.2))");
    }

    #[test]
    fn context_renders_expected_term() {
        assert_eq!(
            good_ctx().to_metta(),
            "(context (measured (rates 0.4 0 0.1)) True True local-brain free)"
        );
    }

    #[test]
    fn parse_verdict_reads_approved_and_rejected() {
        assert_eq!(parse_verdict(&["Approved".to_string()]).unwrap(), Verdict::Approved);
        assert_eq!(
            parse_verdict(&["(Rejected perception)".to_string()]).unwrap(),
            Verdict::Rejected("perception".to_string())
        );
    }

    #[test]
    fn parse_verdict_fails_closed_on_garbage() {
        // Not Approved and not a well-formed Rejected -> error, never a silent approve.
        assert!(parse_verdict(&["Maybe".to_string()]).is_err());
        assert!(parse_verdict(&[]).is_err());
        assert!(parse_verdict(&["Approved".to_string(), "extra".to_string()]).is_err());
    }

    #[test]
    fn gate_approves_when_brain_says_approved() {
        // The gate is transport + parsing; a FakeBrain lets us test that in isolation. We match
        // the fake on the exact program the gate builds so we also lock the program format.
        let action = Setpoint::new(0.5, 0.0, 0.2);
        let ctx = good_ctx();
        let program = format!(
            "PREAMBLE\n!(gate-setpoint {} {})",
            action.to_metta(),
            ctx.to_metta()
        );
        let brain = FakeBrain::new().with_answer(program, vec!["Approved".to_string()]);
        let gate = Gate::new(brain, "PREAMBLE");
        assert_eq!(gate.check(action, &ctx).unwrap(), Verdict::Approved);
    }

    #[test]
    fn gate_surfaces_rejection_reason() {
        let action = Setpoint::new(0.5, 0.0, 0.2);
        let ctx = good_ctx();
        let program = format!(
            "PREAMBLE\n!(gate-setpoint {} {})",
            action.to_metta(),
            ctx.to_metta()
        );
        let brain =
            FakeBrain::new().with_answer(program, vec!["(Rejected communication)".to_string()]);
        let gate = Gate::new(brain, "PREAMBLE");
        let verdict = gate.check(action, &ctx).unwrap();
        assert_eq!(verdict, Verdict::Rejected("communication".to_string()));
        assert!(!verdict.is_approved());
    }

    #[test]
    fn gate_errors_when_brain_returns_nothing() {
        // Unknown query -> FakeBrain returns []; the gate must fail closed, not approve.
        let gate = Gate::new(FakeBrain::new(), "PREAMBLE");
        let err = gate.check(Setpoint::new(0.5, 0.0, 0.2), &good_ctx()).unwrap_err();
        assert!(matches!(err, GateError::Unparseable(_)));
    }
}
