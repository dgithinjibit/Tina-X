# Developer Setup (Phase 0)

Everything below is runnable today. See `ROADMAP.md` for what comes next.

## Prerequisites
- Rust stable (`rustup default stable`) — tested on 1.97
- Python 3.12

## Rust core (fast reflex loop + slow MeTTa brain bridge)
```bash
cargo test  -p nzi-core                    # 10 tests (8 unit + 2 bridge integration)
cargo run   -p nzi-core --bin reflex-demo  # reflex micro-benchmark vs the 13 ms fly budget
cargo run   -p nzi-core --bin brain-demo   # Rust <-> MeTTa bridge (needs the venv below)
cargo clippy -p nzi-core --all-targets     # lint
```
Expected: all tests pass; `reflex-demo` reports "OK ... within the fly budget and converges"
(max step latency single-digit-to-tens of µs — ~900x under the 13 ms budget); `brain-demo`
prints MeTTa results driven from Rust (`!(+ 1 2)` → `3`).

The 2 bridge integration tests **skip gracefully** if the venv isn't set up, so `cargo test`
is green even without MeTTa — but set up the venv below to actually exercise the bridge.

## MeTTa / Hyperon (the slow symbolic brain)
Debian/Ubuntu use PEP 668 (externally-managed Python), so use a venv:
```bash
python3 -m venv .venv
.venv/bin/pip install --upgrade pip
.venv/bin/pip install hyperon          # 0.2.10, pre-alpha

.venv/bin/python metta-logic/run_smoke.py            # smoke test: nzi-001 / True / False
.venv/bin/python metta-logic/bench_metta.py          # latency baseline (P0.3)
.venv/bin/python metta-logic/bridge_worker.py '!(+ 1 2)'  # the worker Rust calls -> {"ok":true,...}
```
With the venv present, re-run `cargo run -p nzi-core --bin brain-demo` and the 2 integration
tests to see Rust drive MeTTa for real.

## What Phase 0 has proven
- Rust ↔ reflex loop: stable, converges, **~900x under** the 13 ms fly-derived budget.
- MeTTa runs from Python. Benchmarked ourselves (no vendor numbers exist):
  - **rule-eval ~2–7 ms p99, independent of Atomspace size** — the slow brain scales.
  - **direct `space.query()` is O(n): ~2 s @100k, ~23 s @1M atoms** — unusable naively at scale
    → drove `docs/adr/0002-partitioned-atomspace.md`.
  - parse is cheap; **evaluate dominates**.
- Both decisions (two-rate brain, partitioned Atomspace) are now backed by measured numbers in
  `docs/benchmarks/metta-baseline.md`.
- **Rust drives MeTTa end-to-end** via `brain::SubprocessBrain` (ADR 0003) — proven by
  `brain-demo` and 2 integration tests.

The full sweep takes a few minutes (1M-atom queries are slow by nature). Use `--quick` to skip 1M.

## Next
Phase 0 is complete (P0.1–P0.5 done). `ROADMAP.md` → **Phase 1**: Unity ML-Agents scene + the
reflex loop against real plant dynamics, then tune the PD gains (P1.2).
