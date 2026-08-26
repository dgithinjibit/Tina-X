# ADR 0006 — Supervisory loop: brain proposes, gate verifies, reflex acts

- **Status:** Accepted
- **Date:** 2026-08-08
- **Phase:** 2 (Stage ②, P2.4–P2.6)

## Context
The two-rate brain (ADR 0001) has a fast reflex loop (<13 ms, `reflex.rs`) and a slow symbolic
brain (MeTTa, milliseconds). Stage ① built the verification gate (`verify.rs`): given a proposed
setpoint + context, it returns `Approved` or `(Rejected <check>)`, catching the five
agent-hallucination types. What was missing is the **control step that connects the two rates**:
who asks the brain for a setpoint, when is the gate consulted, and what happens on refusal.

The danger to design against is not a wrong answer — it's a **confident** wrong answer reaching the
actuators. So the loop must be safe *specifically when it is unsure*.

## Decision
A `Supervisor<B: SymbolicBrain>` (`supervise.rs`) runs the SLOW control step. Each `tick`:

1. **Propose.** Ask the brain to reason over the knowledge base
   (`metta-logic/knowledge/{world,mission,agent}.metta`) and the latest telemetry:
   `!(decide-setpoint <telemetry>)` → `(setpoint (rates R P Y))`. The KB clamps proposals into the
   physical envelope, so a healthy proposal is gate-consistent *by construction*.
2. **Verify.** Run that proposal through the Stage-① gate, stamping the agent's own trusted
   identity (`local-brain`, from `agent.metta`, on the verification allow-list) as the source.
3. **Act, or hold.** On `Approved`, update the reflex target and the **last-known-safe** setpoint.
   On anything else, keep holding last-safe.

The gate sits *between* the two rates. Both the propose and verify steps are MeTTa calls (ms), so
the whole supervisor is a SLOW-loop component and is **never** invoked from the <13 ms reflex path.
The reflex loop keeps holding the last approved setpoint between ticks.

### Governance is fail-closed
The supervisor retains the last-known-safe setpoint (initialized to zero — "hold still") and
replaces it **only** with another *approved* setpoint. Every non-approval holds last-safe, with the
reason recorded:

- `Refused(<check>)` — the gate rejected the proposal (perception/reasoning/execution/…).
- `HeldOnDoubt(<reason>)` — the brain returned nothing, an unparseable proposal, or the call
  errored. An unclear answer is treated as "do not act", never as approval.

## Consequences
- (+) The verification moat is now *in the control path*: no setpoint reaches the reflex loop
  without an `Approved` verdict.
- (+) Every tick yields a `telemetry::BrainDecision { query, results, verified }` — the reasoning
  is dashboard-visible (Stage ② wires the live stream; the flag already exists on the wire).
- (+) Testable without MeTTa (`FakeBrain` state-machine tests) and proven with it (venv-backed
  `supervise_integration.rs`, skips gracefully).
- (−) Two brain calls per tick (propose + verify) — fine for the slow loop; a future long-lived
  MeTTa worker (see ADR 0003) removes the per-call subprocess startup.
- (−) The KB reloads rules text each tick from disk via the loader; acceptable now, cache later if
  the supervisory tick rate rises.

## Validation
- `cargo test -p tina-core` — supervisor unit tests (approve→forward, refuse→hold, doubt→hold,
  parser) + `supervise_integration.rs` (real KB→gate: free approved, hold-level→zero approved,
  stale telemetry refused-and-held).
- `cargo run -p tina-core --bin supervise-demo` — a 4-tick scenario showing approvals forwarded and
  a stale-telemetry tick refused with last-safe held.
- `python3 metta-logic/run_supervise_smoke.py` — the KB `decide-setpoint` rule in isolation.
