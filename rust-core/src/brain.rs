//! The "slow brain" bridge: how Rust asks MeTTa (Hyperon) questions.
//!
//! # Why this module exists (read this first if you're new)
//!
//! Project Nzi has a **two-rate brain** (see `docs/adr/0001-two-rate-brain.md`):
//! - a FAST reflex loop in [`crate::reflex`] (microseconds, control stability), and
//! - a SLOW symbolic brain in MeTTa/Hyperon (milliseconds, reasoning & verification).
//!
//! Rust can't yet link the MeTTa library directly — the `hyperon` crate isn't on crates.io
//! and the pip package only ships a Python extension (see `docs/adr/0003-rust-metta-bridge.md`).
//! So we define a small, stable **trait** ([`SymbolicBrain`]) that the rest of the codebase
//! programs against, and provide two implementations:
//!
//! - [`FakeBrain`]      — an in-memory test double (no MeTTa needed; great for unit tests).
//! - [`SubprocessBrain`] — the real thing: it runs `metta-logic/bridge_worker.py` as a
//!   subprocess and parses the JSON it prints back.
//!
//! When a native MeTTa Rust binding becomes viable later, it becomes *just another* impl of
//! [`SymbolicBrain`] — nothing that calls the brain has to change.
//!
//! # IMPORTANT rule
//! The brain lives in the SLOW loop only. **Never call it from the reflex loop** — a query
//! costs milliseconds (see `docs/benchmarks/metta-baseline.md`), which is far too slow for the
//! <13 ms reflex budget.

use std::path::PathBuf;
use std::process::Command;

/// A question we ask the symbolic brain.
///
/// Right now this is just MeTTa source text (e.g. `"!(safe-to-fly? 8)"`). It's a struct rather
/// than a bare `String` so we can add fields later (timeouts, which sub-space to query, etc.)
/// without breaking every caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MettaQuery {
    /// The MeTTa program/query text to evaluate.
    pub text: String,
}

impl MettaQuery {
    /// Convenience constructor so callers can write `MettaQuery::new("!(+ 1 2)")`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// The answer coming back from the symbolic brain.
///
/// `results` is the list of atoms MeTTa produced, each already turned into a string at the
/// boundary (the Rust side stays language-neutral and doesn't need to model MeTTa atoms).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MettaResult {
    pub results: Vec<String>,
}

/// Something that can answer MeTTa queries. This is the seam the whole slow-loop uses.
///
/// Keeping it a trait means: unit tests use [`FakeBrain`], production uses [`SubprocessBrain`],
/// and a future native binding is a drop-in third impl — callers never change.
pub trait SymbolicBrain {
    /// Ask one query. Returns the results, or an error describing what went wrong.
    fn query(&self, query: &MettaQuery) -> Result<MettaResult, BrainError>;
}

/// Everything that can go wrong when talking to the brain.
///
/// We keep this a small, explicit enum (rather than a stringly-typed error) so callers can
/// match on the *kind* of failure and react differently (e.g. retry vs. give up).
#[derive(Debug)]
pub enum BrainError {
    /// We couldn't even start the worker process (e.g. Python missing, bad path).
    Spawn(String),
    /// The worker ran but reported a failure (its JSON said `"ok": false`).
    Worker(String),
    /// The worker's output wasn't the JSON we expected (contract violation).
    Protocol(String),
}

impl std::fmt::Display for BrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrainError::Spawn(m) => write!(f, "failed to start MeTTa worker: {m}"),
            BrainError::Worker(m) => write!(f, "MeTTa worker reported an error: {m}"),
            BrainError::Protocol(m) => write!(f, "MeTTa worker sent unexpected output: {m}"),
        }
    }
}

impl std::error::Error for BrainError {}

// ---------------------------------------------------------------------------------------------
// FakeBrain — an in-memory test double.
// ---------------------------------------------------------------------------------------------

/// A pretend brain that returns canned answers. Use this in tests for anything that *uses* a
/// brain, so those tests don't need MeTTa installed and run in microseconds.
///
/// It matches on the exact query text; unknown queries return an empty result list.
#[derive(Default, Clone)]
pub struct FakeBrain {
    /// (query text) -> (results to return). A plain Vec keeps it dependency-free and obvious.
    canned: Vec<(String, Vec<String>)>,
}

impl FakeBrain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Teach the fake brain to answer `text` with `results`. Returns `self` for chaining.
    pub fn with_answer(mut self, text: impl Into<String>, results: Vec<String>) -> Self {
        self.canned.push((text.into(), results));
        self
    }
}

impl SymbolicBrain for FakeBrain {
    fn query(&self, query: &MettaQuery) -> Result<MettaResult, BrainError> {
        for (text, results) in &self.canned {
            if *text == query.text {
                return Ok(MettaResult { results: results.clone() });
            }
        }
        // Unknown query: return "no results" rather than erroring — tests decide what that means.
        Ok(MettaResult { results: Vec::new() })
    }
}

// ---------------------------------------------------------------------------------------------
// SubprocessBrain — the real bridge to MeTTa via the Python worker.
// ---------------------------------------------------------------------------------------------

/// Talks to MeTTa by running `metta-logic/bridge_worker.py` with the venv's Python.
///
/// See `docs/adr/0003-rust-metta-bridge.md` for why we use a subprocess instead of linking
/// MeTTa directly. This is fine because the brain is a SLOW-loop component.
#[derive(Clone)]
pub struct SubprocessBrain {
    /// Path to the Python interpreter (typically the project venv: `.venv/bin/python`).
    python: PathBuf,
    /// Path to `bridge_worker.py`.
    worker: PathBuf,
}

impl SubprocessBrain {
    /// Build a brain from explicit paths. Prefer [`SubprocessBrain::from_project_root`] unless
    /// you have a reason to point at a specific interpreter.
    pub fn new(python: impl Into<PathBuf>, worker: impl Into<PathBuf>) -> Self {
        Self { python: python.into(), worker: worker.into() }
    }

    /// Build a brain given the project root, using the conventional `.venv` and worker paths.
    /// This encodes the layout documented in `docs/DEV_SETUP.md`.
    pub fn from_project_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let python = root.join(".venv").join("bin").join("python");
        let worker = root.join("metta-logic").join("bridge_worker.py");
        Self::new(python, worker)
    }

    /// Returns true if both the Python interpreter and the worker script exist on disk.
    /// Tests use this to SKIP gracefully when the venv hasn't been set up.
    pub fn is_available(&self) -> bool {
        self.python.exists() && self.worker.exists()
    }
}

impl SymbolicBrain for SubprocessBrain {
    fn query(&self, query: &MettaQuery) -> Result<MettaResult, BrainError> {
        // 1. Run: `<python> <worker> "<query text>"`  and capture stdout.
        let output = Command::new(&self.python)
            .arg(&self.worker)
            .arg(&query.text)
            .output()
            .map_err(|e| BrainError::Spawn(format!("{}: {e}", self.python.display())))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 2. The worker promises EXACTLY one JSON line (see bridge_worker.py). Take the last
        //    non-empty line to be robust against any stray leading output.
        let json_line = stdout
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .ok_or_else(|| {
                let stderr = String::from_utf8_lossy(&output.stderr);
                BrainError::Protocol(format!("worker produced no output (stderr: {stderr})"))
            })?;

        // 3. Parse the JSON contract: {"ok": bool, "results": [...], "error": "..."}.
        let parsed: serde_json::Value = serde_json::from_str(json_line)
            .map_err(|e| BrainError::Protocol(format!("invalid JSON `{json_line}`: {e}")))?;

        let ok = parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let msg = parsed
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(BrainError::Worker(msg));
        }

        // 4. Pull out the "results" array, converting each element to a String.
        let results = parsed
            .get("results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| BrainError::Protocol("missing `results` array".to_string()))?
            .iter()
            .map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()))
            .collect();

        Ok(MettaResult { results })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_brain_returns_canned_answer() {
        let brain = FakeBrain::new().with_answer("!(safe? 8)", vec!["True".to_string()]);
        let out = brain.query(&MettaQuery::new("!(safe? 8)")).unwrap();
        assert_eq!(out.results, vec!["True".to_string()]);
    }

    #[test]
    fn fake_brain_unknown_query_is_empty() {
        let brain = FakeBrain::new();
        let out = brain.query(&MettaQuery::new("!(anything)")).unwrap();
        assert!(out.results.is_empty());
    }

    #[test]
    fn brain_error_displays_readably() {
        // A junior dev debugging a failure should see a clear message, not a Debug blob.
        let e = BrainError::Worker("boom".to_string());
        assert_eq!(e.to_string(), "MeTTa worker reported an error: boom");
    }

    #[test]
    fn subprocess_brain_reports_unavailable_for_bad_paths() {
        let brain = SubprocessBrain::new("/no/such/python", "/no/such/worker.py");
        assert!(!brain.is_available());
    }

    #[test]
    fn subprocess_brain_spawn_error_when_python_missing() {
        let brain = SubprocessBrain::new("/definitely/not/python", "/tmp/whatever.py");
        let err = brain.query(&MettaQuery::new("!(+ 1 2)")).unwrap_err();
        matches!(err, BrainError::Spawn(_));
    }
}
