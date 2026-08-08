# ADR 0002 — Partitioned Atomspace (no naive full-space queries at scale)

- **Status:** Accepted (driven by P0.3 benchmark evidence)
- **Date:** 2026-08-08
- **Phase:** 0 (P0.3 outcome)

## Context
Our own P0.3 benchmarks (`docs/benchmarks/metta-baseline.md`, hyperon 0.2.10) measured direct
`space.query()` against the default flat `GroundingSpace` as **~O(n)**:

| atoms | query p50 |
|---|---|
| 1,000 | 19 ms |
| 100,000 | 2,025 ms |
| 1,000,000 | 23,000 ms |

By contrast, **rule evaluation stayed ~2–7 ms p99 regardless of size**, because a rule that
doesn't scan the fact base isn't penalized by base size.

No vendor benchmarks existed; we found this ourselves. It is a hard real-time blocker if we
naively store all agent/world facts in one space and pattern-query it.

## Decision
1. **Never rely on naive full-space `query()` over a large flat Atomspace in any hot path.**
2. **Partition the Atomspace by concern and recency:**
   - a small, hot "working space" per agent (current state, recent telemetry) — kept tiny so
     any query stays in the fast regime (<~20 ms);
   - larger, colder spaces for archival/world knowledge, reasoned over via **rules** (constant
     cost) rather than scanned via `query()`.
3. **Prefer rule-driven inference over pattern scans** wherever possible.
4. **Evaluate the MORK "zipper machine" path** (claimed orders-of-magnitude faster matching) and
   re-benchmark before committing to any large-space query design.

## Consequences
- (+) Keeps symbolic reasoning latency bounded and size-independent for the common case.
- (+) Aligns with the two-rate brain (ADR 0001): even the slow loop stays responsive.
- (−) We must design an explicit data-lifecycle: what's promoted to the hot space, what's
  archived, and how rules bridge them. (Phase 2 work.)
- (−) Adds a benchmarking obligation before any feature that queries a large space.

## Validation
- Re-run the P0.3 query sweep after prototyping partitioned/indexed spaces; the goal is query
  p99 that does NOT grow with total-knowledge size.
- **DONE (P2.7, 2026-08-08).** `metta-logic/bench_partition.py` compares a flat `query()` against a
  separate tiny hot-space `query()` at equal total knowledge. Result: hot-space p99 stayed
  **0.9 ms → 0.6 ms** across 1k → 100k (0.7×, flat) while flat `query()` grew **92.6×**. Goal met:
  partitioned query p99 is size-independent. The bound is enforced in Rust by
  `atomspace::HotWorkingSpace::MAX_FACTS`. Full numbers in `docs/benchmarks/metta-baseline.md`.
