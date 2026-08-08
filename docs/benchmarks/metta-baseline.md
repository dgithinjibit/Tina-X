# MeTTa/Hyperon Baseline Benchmarks

> **Why this file exists:** verified research found NO published latency/throughput numbers
> for Hyperon/MeTTa. Every real-time architecture decision in Nzi must cite a number we
> measured ourselves. This is the living record. (ROADMAP P0.3)

## Environment
- hyperon **0.2.10** (pip, pre-alpha), Python 3.12.3, Linux x86_64
- Rust 1.97.1
- Date of first run: 2026-08-08

## Run 1 — trivial query, tiny knowledge base (P0.3 seed)
Command: `.venv/bin/python metta-logic/bench_metta.py` (1000 iterations of `!(safe-to-fly? W)`)

| metric | latency |
|---|---|
| p50 | **2.46 ms** |
| p95 | 4.10 ms |
| p99 | **5.49 ms** |
| mean | 2.67 ms |

## First-look interpretation ⚠️
- A **trivial** MeTTa query already costs p99 ≈ 5.5 ms — **~40% of the 13 ms reflex budget**
  (`REFLEX_BUDGET_US`) on a toy problem with a near-empty Atomspace.
- **This is direct empirical support for ADR 0001 (the two-rate brain):** symbolic reasoning
  must NEVER run inside the reflex loop. The fast loop (measured at **max 14 µs**, ~900× under
  budget — see `reflex-demo`) and the slow MeTTa brain are correctly separated.
- Caveat: `metta.run(str)` includes parse overhead per call. A fair next measurement should
  pre-compile/reuse the query and separate parse time from evaluation time.

## Run 2 — full P0.3: parse-vs-eval split + Atomspace-size sweep
Command: `.venv/bin/python metta-logic/bench_metta.py`

### Parse vs Evaluate (tiny Atomspace, n=1000)
| stage | p50 | p95 | p99 | max |
|---|---|---|---|---|
| parse-only | 0.133 ms | 0.379 ms | 0.632 ms | 2.96 ms |
| **evaluate-only** (pre-parsed) | **2.325 ms** | 7.767 ms | **10.101 ms** | 27.66 ms |

→ **Parsing is cheap; evaluation dominates.** `metta.run(str)` cost is ~95% evaluate, ~5% parse.

### Atomspace-size sweep (rule-eval vs direct pattern query)
| operation | size | p50 | p95 | p99 | max |
|---|---|---|---|---|---|
| rule-eval | 1,000 | 2.10 ms | 4.16 ms | 5.07 ms | 6.06 ms |
| rule-eval | 100,000 | 3.19 ms | 4.43 ms | 7.23 ms | 10.7 ms |
| rule-eval | 1,000,000 | 2.77 ms | 4.40 ms | 6.31 ms | 130.6 ms |
| **query** (`space.query`) | 1,000 | 19.3 ms | 40.7 ms | 91.4 ms | 141.7 ms |
| **query** | 100,000 | **2,025 ms** | 3,305 ms | 3,305 ms | 3,305 ms |
| **query** | 1,000,000 | **23,000 ms** | 33,485 ms | 33,485 ms | 33,485 ms |

Bulk `add_atom` load time: 1k = 0.05s, 100k = 4.1s, 1M = 37.8s.

## 🔑 Headline findings (these shape the architecture)
1. **Rule evaluation latency is ~constant (~2–7 ms p99) regardless of Atomspace size.**
   A rule that doesn't scan facts is not penalized by a large knowledge base. GOOD — the slow
   brain can hold a big world model and still reason in single-digit ms.
2. **Direct `space.query()` on the default flat GroundingSpace is ~O(n) and catastrophic:**
   ~2 s at 100k atoms, ~23 s at 1M atoms. **Unusable for real-time as-is.**
   → **Architecture consequence:** we must NOT rely on naive full-space pattern queries at scale.
   Options to build/evaluate (feeds ROADMAP): indexed sub-spaces, partitioned Atomspaces per
   concern, keeping hot facts in a small space, or the MORK "zipper machine" path (claims
   orders-of-magnitude faster matching — verify ourselves when accessible).
3. **Parsing is negligible; evaluation is the cost.** Pre-parse/cache atoms in hot paths.
4. **ADR 0001 confirmed with hard numbers:** even the *cheapest* symbolic op (evaluate, p99
   10 ms) is ~78% of the 13 ms reflex budget; the worst (query @1M, 33 s) is 2,500× over it.
   Symbolic reasoning stays in the SLOW loop, full stop.

## P2.7 — the O(n) fix, measured (`bench_partition.py`, hyperon 0.2.10)
The ADR 0002 fix, validated: keep a **separate tiny hot space** and query it instead of scanning a
big flat space. Same total knowledge loaded in both cases; the partitioned path queries only the
hot space.

| total knowledge | FLAT `query()` p99 | PARTITIONED hot-space `query()` p99 |
|---|---|---|
| 1,000 | 107 ms | 0.91 ms |
| 100,000 | 9,936 ms | 0.61 ms |
| **growth (1k→100k)** | **92.6×** | **0.7× (flat)** |

**Conclusion:** the hot-space query is sub-millisecond and does NOT grow with total-knowledge size,
while flat `query()` grows ~O(n) as before. This retires open-risk #2 for the common case. (1M flat
point omitted — ~23 s/query, and the trend is already unambiguous.) The bound that keeps it flat is
enforced in Rust by `atomspace::HotWorkingSpace::MAX_FACTS`. NOTE: building ~100k keyed rules in one
`m.run()` panics hyperon's trie index (pre-alpha bug); load large tables as facts, not one program.

## TODO (remaining, tracked)
- [x] Separate parse vs. evaluate time.
- [x] Sweep Atomspace sizes 1e3 / 1e5 / 1e6.
- [x] Fix the O(n) query problem: partitioned hot/cold spaces; re-benchmarked (P2.7, above).
- [ ] Benchmark a realistic multi-step / backward-chaining query (not just a lookup).
- [ ] Evaluate MORK path when accessible (verify the "orders of magnitude" claim ourselves).
- [ ] Measure the Rust `libhyperon` crate path latency vs. the Python path.
