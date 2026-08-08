# Developer Setup (Phase 0)

Everything below is runnable today. See `ROADMAP.md` for what comes next.

## Prerequisites
- Rust stable (`rustup default stable`) — tested on 1.97
- Python 3.12

## Rust core (the fast reflex loop)
```bash
cargo test -p nzi-core           # 3 tests: zero-error, convergence, no-NaN regression
cargo run  -p nzi-core --bin reflex-demo   # micro-benchmark vs the 13 ms fly budget
cargo clippy -p nzi-core         # lint
```
Expected: all tests pass; demo reports "OK — reflex loop is within the fly budget and converges"
(max step latency single-digit-to-tens of µs — ~900x under the 13 ms budget).

## MeTTa / Hyperon (the slow symbolic brain)
Debian/Ubuntu use PEP 668 (externally-managed Python), so use a venv:
```bash
python3 -m venv .venv
.venv/bin/pip install --upgrade pip
.venv/bin/pip install hyperon          # 0.2.10, pre-alpha

.venv/bin/python metta-logic/run_smoke.py     # smoke test: expect nzi-001 / True / False
.venv/bin/python metta-logic/bench_metta.py   # latency baseline (P0.3)
```

## What Phase 0 has proven
- Rust ↔ reflex loop: stable, converges, **~900x under** the 13 ms fly-derived budget.
- MeTTa runs from Python. Benchmarked ourselves (no vendor numbers exist):
  - **rule-eval ~2–7 ms p99, independent of Atomspace size** — the slow brain scales.
  - **direct `space.query()` is O(n): ~2 s @100k, ~23 s @1M atoms** — unusable naively at scale
    → drove `docs/adr/0002-partitioned-atomspace.md`.
  - parse is cheap; **evaluate dominates**.
- Both decisions (two-rate brain, partitioned Atomspace) are now backed by measured numbers in
  `docs/benchmarks/metta-baseline.md`.

The full sweep takes a few minutes (1M-atom queries are slow by nature). Use `--quick` to skip 1M.

## Next
`ROADMAP.md` → finish P0.3 (Atomspace-size sweep), then Phase 1 (Unity + real plant dynamics).
```
```
