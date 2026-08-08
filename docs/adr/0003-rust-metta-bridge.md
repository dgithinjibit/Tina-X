# ADR 0003 — Rust↔MeTTa Bridge: trait seam now, native FFI later

- **Status:** Accepted
- **Date:** 2026-08-08
- **Phase:** 0 (P0.4)

## Context
Phase 0's goal was to prove Rust can talk to MeTTa (the slow symbolic brain). While
attempting the "obvious" path — depend on a `hyperon` Rust crate — we found:

- **`hyperon` is NOT published on crates.io** (`cargo add hyperon` → "could not be found in
  registry index"). It exists only as a git crate inside `trueagi-io/hyperon-experimental`.
- The pip package ships only a **Python C-extension** (`hyperonpy...so`) — it does **not**
  expose a standalone `libhyperon` C library we can link against directly.

So a "real" native binding today means either (a) a **git dependency** that builds the entire
pre-alpha Rust workspace from source (heavy, network-dependent, unstable API), or (b) hand-written
**FFI** against a C API we'd have to build ourselves. Neither is a good Phase-0 commitment, and
both would couple our core to a moving pre-alpha target before we even have one working agent.

## Decision
Introduce a **narrow trait seam** in `rust-core` that the whole slow-loop programs against:

- `SymbolicBrain` — a trait with a single job: take a query, return a result.
- `MettaQuery` / `MettaResult` — plain data types crossing the boundary.
- `SubprocessBrain` — an implementation that shells out to our **already-proven** Python MeTTa
  path (the venv + `hyperonpy`). This works TODAY and reuses verified code.
- `FakeBrain` — an in-memory test double, so all logic above the brain is testable without MeTTa.

The native `libhyperon`/FFI implementation becomes just **another impl of the same trait** later,
with zero changes to callers.

## Why this is the right Phase-0 call
- **It genuinely proves Rust↔MeTTa interop** (Rust drives a MeTTa query and gets a typed result),
  which is what P0.4 asked for.
- **It doesn't fake a dependency we can't fetch**, and doesn't chain our core to a pre-alpha crate.
- **The seam is the real long-term interface** — swapping in FFI later is a drop-in, not a rewrite.

## Consequences
- (+) Testable now (FakeBrain), runnable now (SubprocessBrain), future-proof (trait).
- (+) Latency cost of the subprocess is fine: the brain lives in the SLOW loop (ADR 0001), and we
  already measured MeTTa eval at ms scale (ADR 0002 / benchmarks) — subprocess startup is the
  main extra cost and is acceptable for a supervisory loop, not the reflex loop.
- (−) Subprocess startup adds overhead per call; a long-lived worker or native FFI will replace it.
  Tracked as future work (P6 / when the native crate path is viable).

## Validation
- Unit tests use `FakeBrain` (no MeTTa needed).
- An integration test runs the real `SubprocessBrain` end-to-end, and **skips gracefully** if the
  venv/hyperon is not present (so CI without MeTTa still passes).
