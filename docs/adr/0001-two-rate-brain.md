# ADR 0001 — The Two-Rate Brain

- **Status:** Accepted
- **Date:** 2026-08-08
- **Phase:** 0 (P0.5)

## Context
Autonomous agents fail on *execution*, not intelligence (see `limitations-edge-cases/`).
Two verified constraints collide:
1. Robot control loops need **>50 Hz** for stability.
2. Foundation-model / heavy symbolic inference adds **100s of ms to seconds** of latency.

Running reasoning inside the control loop therefore destabilizes the agent. Biology already
solved this: the fly stabilizes via a fast reflex (halteres, ~5–13 ms, modeled as PD-with-delay)
while slower brain circuits handle navigation and decisions (see `fly-biomimicry/`).

## Decision
Split every Nzi agent into two loops:

- **Fast reflex loop** (`rust-core::reflex`): delayed-PD stabilizer, allocation-free,
  budgeted under `REFLEX_BUDGET_US` (13 ms), targeting `REFLEX_TARGET_HZ` (500 Hz).
- **Slow symbolic brain** (`metta-logic/`): MeTTa/Hyperon reasoning, verification, and
  swarm coordination. It emits *setpoints* to the reflex loop and reads *telemetry* back.

**Hard rule: symbolic inference NEVER executes inside the reflex path.**

## Consequences
- (+) We can adopt slow, powerful symbolic reasoning without risking control stability.
- (+) The reflex loop is embeddable/WASM-able independently of the (heavy) MeTTa runtime.
- (+) Clean seam for the verification moat: the brain proposes, the reflex disposes.
- (−) We need a well-defined setpoint/telemetry interface between the two rates (Phase 2).
- (−) Requires measuring both sides' latency continuously (P0.3 + benchmarks track).

## Validation
- P0.3 benchmarks MeTTa/MORK latency (is the "slow" side slow enough to matter? how slow?).
- `reflex-demo` binary measures the fast side against the 13 ms budget from day one.
