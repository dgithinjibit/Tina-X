#!/usr/bin/env python3
"""Project TINA-X — MeTTa/Hyperon latency benchmark (P0.3, full).

The single most important Phase-0 de-risking step: MEASURE MeTTa latency before we lock in
any real-time architecture. No vendor benchmarks exist (verified in research), so we produce
our own and record them in docs/benchmarks/metta-baseline.md.

This measures three things the SLOW symbolic brain will actually do (never inside the <13 ms
reflex loop — see docs/adr/0001-two-rate-brain.md):
  1. PARSE vs EVALUATE split         — where does the time go in `metta.run(str)`?
  2. Rule evaluation latency         — `!(safe-to-fly? W)` as the Atomspace grows.
  3. Direct pattern-query latency    — `space.query(...)` as the Atomspace grows.

Atomspace-size sweep: 1e3 / 1e5 / 1e6 atoms.

Run:  .venv/bin/python metta-logic/bench_metta.py
      .venv/bin/python metta-logic/bench_metta.py --quick   # 1e3/1e5 only (fast)
"""
from __future__ import annotations

import statistics
import sys
import time


def pct(sorted_ms: list[float], p: float) -> float:
    if not sorted_ms:
        return float("nan")
    idx = min(len(sorted_ms) - 1, int(p * len(sorted_ms)))
    return sorted_ms[idx]


def summarize(name: str, samples_ms: list[float]) -> dict:
    s = sorted(samples_ms)
    return {
        "name": name,
        "n": len(s),
        "p50": statistics.median(s),
        "p95": pct(s, 0.95),
        "p99": pct(s, 0.99),
        "mean": statistics.mean(s),
        "max": s[-1],
    }


def print_row(r: dict) -> None:
    print(
        f"  {r['name']:<34} n={r['n']:<6} "
        f"p50={r['p50']:.3f}  p95={r['p95']:.3f}  p99={r['p99']:.3f}  "
        f"mean={r['mean']:.3f}  max={r['max']:.3f}  (ms)"
    )


def bench_parse_vs_eval(MeTTa, n: int = 1000) -> tuple[dict, dict]:
    """Separate parse time from evaluate time (metta.run bundles both)."""
    m = MeTTa()
    m.run(
        "(= (max-wind-tolerance-ms) 12)"
        "(= (safe-to-fly? $w) (if (<= $w (max-wind-tolerance-ms)) True False))"
    )

    parse_ms: list[float] = []
    eval_ms: list[float] = []
    for i in range(n):
        w = i % 25
        text = f"(safe-to-fly? {w})"

        t0 = time.perf_counter()
        atom = m.parse_all(text)[0]
        parse_ms.append((time.perf_counter() - t0) * 1000.0)

        t1 = time.perf_counter()
        m.evaluate_atom(atom)
        eval_ms.append((time.perf_counter() - t1) * 1000.0)

    return summarize("parse-only", parse_ms), summarize("evaluate-only (pre-parsed)", eval_ms)


def bench_size_sweep(MeTTa, E, S, ValueAtom, sizes: list[int]) -> list[dict]:
    """For each Atomspace size, measure rule-eval and direct-query latency.

    WARNING (measured): direct `space.query()` against the default flat GroundingSpace is
    ~O(n) — it costs seconds per query at 1e5+ atoms. So we run MANY rule-evals (cheap, and
    independent of space size) but only a FEW direct queries at large sizes, or the sweep
    would take hours. This asymmetry is itself a headline result — see the benchmark doc.
    """
    rows: list[dict] = []
    for size in sizes:
        m = MeTTa()
        m.run(
            "(= (max-wind-tolerance-ms) 12)"
            "(= (safe-to-fly? $w) (if (<= $w (max-wind-tolerance-ms)) True False))"
        )
        sp = m.space()

        # Bulk-load `size` distractor facts so the Atomspace is realistically large.
        t_load = time.perf_counter()
        for i in range(size):
            sp.add_atom(E(S("fact"), ValueAtom(i)))
        load_s = time.perf_counter() - t_load
        print(f"  [size={size:>9,}]  loaded in {load_s:6.2f}s  (atom_count={sp.atom_count():,})", flush=True)

        # 1) rule evaluation as the space grows (cheap; size-independent). Many samples.
        n_rule = 500
        rule_ms: list[float] = []
        for i in range(n_rule):
            atom = m.parse_all(f"(safe-to-fly? {i % 25})")[0]
            t0 = time.perf_counter()
            m.evaluate_atom(atom)
            rule_ms.append((time.perf_counter() - t0) * 1000.0)
        r = {**summarize(f"rule-eval @ {size:,}", rule_ms), "size": size}
        print_row(r)
        rows.append(r)

        # 2) direct pattern query as the space grows. Scale sample count DOWN with size to
        #    keep wall-clock sane, because each query is ~O(n).
        n_query = 200 if size <= 1_000 else (20 if size <= 100_000 else 5)
        query_ms: list[float] = []
        for i in range(n_query):
            target = (i * 7) % size if size else 0
            pat = E(S("fact"), ValueAtom(target))
            t0 = time.perf_counter()
            sp.query(pat)
            query_ms.append((time.perf_counter() - t0) * 1000.0)
        q = {**summarize(f"query @ {size:,}", query_ms), "size": size}
        print_row(q)
        rows.append(q)

    return rows


def main() -> int:
    # Line-buffer stdout so progress is visible when piped/redirected (long-running sweep).
    try:
        sys.stdout.reconfigure(line_buffering=True)  # py3.7+
    except Exception:
        pass
    quick = "--quick" in sys.argv
    try:
        from hyperon import MeTTa, E, S, ValueAtom  # type: ignore
    except ModuleNotFoundError:
        print("ERROR: hyperon not installed. See metta-logic/run_smoke.py for setup.", file=sys.stderr)
        return 1

    import hyperon  # type: ignore
    version = getattr(hyperon, "__version__", "unknown")

    print(f"Project TINA-X — MeTTa latency benchmark (P0.3)  [hyperon {version}]")
    print()
    print("== Parse vs Evaluate split (tiny Atomspace) ==")
    parse_r, eval_r = bench_parse_vs_eval(MeTTa)
    print_row(parse_r)
    print_row(eval_r)
    print()

    sizes = [1_000, 100_000] if quick else [1_000, 100_000, 1_000_000]
    print(f"== Atomspace-size sweep {sizes} ==")
    rows = bench_size_sweep(MeTTa, E, S, ValueAtom, sizes)
    print()
    for r in rows:
        print_row(r)
    print()

    # Verdict against the reflex budget.
    budget_ms = 13.0
    worst_eval = max((r["p99"] for r in rows if r["name"].startswith("rule-eval")), default=0.0)
    worst_query = max((r["p99"] for r in rows if r["name"].startswith("query")), default=0.0)
    print("== Verdict vs ADR 0001 (two-rate brain) ==")
    print(f"  reflex budget            : {budget_ms:.1f} ms/step")
    print(f"  worst rule-eval p99      : {worst_eval:.3f} ms  ({worst_eval/budget_ms*100:.0f}% of budget)")
    print(f"  worst direct-query p99   : {worst_query:.3f} ms  ({worst_query/budget_ms*100:.0f}% of budget)")
    print("  Conclusion: symbolic reasoning stays in the SLOW loop. Numbers -> docs/benchmarks/metta-baseline.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
