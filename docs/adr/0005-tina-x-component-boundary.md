# ADR 0005 — TINA-X is an independent component of TINA-X

- **Status:** Accepted
- **Date:** 2026-08-08
- **Phase:** TINA-X foundation (flagship application)

## Context
TINA-X is a symbolic **cascading-failure reasoner**: it ingests infrastructure dependencies
(hospitals, power grids, roads, fuel, substations) into a MeTTa Atomspace and uses forward-chaining
rules to deduce catastrophic **cascades** from compound "black swan" events (earthquake → grid down
→ generator needs fuel → fuel needs road → road flooded by typhoon → hospital fails). This is where
symbolic AI beats deep learning: it reasons about *out-of-distribution* combinations it has never
seen, via the dependency graph.

The user's explicit requirement: **TINA-X must be a COMPONENT of TINA-X, able to communicate with TINA-X,
but totally independent** — a developer must be able to work on and run TINA-X standalone, without
pulling all of TINA-X with it.

## Decision
Build TINA-X in a **self-contained `tina-x/` directory** with:
- its **own** MeTTa logic, Python runner, tests, venv, and README;
- **no hard dependency** on the TINA-X Rust crates, server, or frontend;
- a **thin, OPTIONAL** one-way bridge (`tina-x/bridge/`) that *can* push alerts to the TINA-X
  dashboard when TINA-X is running, but TINA-X runs fully without it.

It **reuses PATTERNS** from TINA-X (the `SymbolicBrain` subprocess approach, the partitioned-Atomspace
discipline from ADR 0002, the "prove the decision" idea) but **copies the small pieces it needs**
rather than importing TINA-X modules. Duplication of a few tiny helpers is the acceptable price of
independence.

## Boundary rules (the contract)
1. `cd tina-x && <setup> && <test>` must work with TINA-X entirely absent.
2. TINA-X may SEND to TINA-X (alerts → dashboard) via a documented HTTP/JSON contract; it must never
   REQUIRE TINA-X to be present.
3. Shared *ideas* (partitioned Atomspace, verifiable decisions) are re-implemented locally in
   TINA-X, not linked. If real shared code emerges later, extract a small published library then —
   not now.
4. TINA-X's MeTTa rules mirror the same "prove the reasoning" ethos: every predicted failure is
   traceable to the rule + facts that produced it (explainable alerts for emergency managers).

## Consequences
- (+) A contributor can own TINA-X end-to-end without learning the whole TINA-X stack.
- (+) TINA-X can be demoed, tested, and deployed on its own (great for a focused pitch).
- (+) Clear seam: TINA-X = the platform (edge agents, swarm, symbolic brain, ZK); TINA-X = its first
  flagship application (societal-fragility digital twin). Weather nowcasting becomes one INPUT.
- (−) Some tiny helpers are duplicated between TINA-X and TINA-X (accepted, per rule 3).
- (−) Two Python environments to manage. Documented in each README.

## Validation
- TINA-X test suite runs green with the TINA-X crates uncompiled.
- The optional bridge is exercised only when `--tina-url` is provided; its absence changes nothing.
