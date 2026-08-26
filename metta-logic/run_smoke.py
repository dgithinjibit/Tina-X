#!/usr/bin/env python3
"""Project TINA-X — MeTTa/Hyperon smoke test harness (P0.2).

Loads `smoke_test.metta`, runs it through the Hyperon MeTTa interpreter, and prints
the results. This is the first proof that our symbolic layer works end-to-end.

Note (from research): Hyperon's Python bindings are pybind11 over a C API (NOT PyO3),
and Hyperon is officially pre-alpha. Install with:  python3 -m pip install --user hyperon

Run:  python3 metta-logic/run_smoke.py
"""
from __future__ import annotations

import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
SMOKE = HERE / "smoke_test.metta"


def main() -> int:
    try:
        from hyperon import MeTTa  # type: ignore
    except ModuleNotFoundError:
        print("ERROR: hyperon is not installed.", file=sys.stderr)
        print("Install it with:  python3 -m pip install --user hyperon", file=sys.stderr)
        return 1

    try:
        import hyperon  # type: ignore
        version = getattr(hyperon, "__version__", "unknown")
    except Exception:
        version = "unknown"

    print(f"Project TINA-X — MeTTa smoke test (hyperon {version})")

    src = SMOKE.read_text()
    metta = MeTTa()

    t0 = time.perf_counter()
    results = metta.run(src)
    elapsed_ms = (time.perf_counter() - t0) * 1000.0

    print(f"  loaded : {SMOKE.name}")
    print(f"  run time: {elapsed_ms:.2f} ms")
    print("  results (one list per '!' query):")
    for i, r in enumerate(results):
        print(f"    [{i}] {r}")

    # Basic sanity: we expect 3 query results back.
    ok = len(results) == 3
    print("  RESULT: OK" if ok else f"  RESULT: REVIEW (expected 3 query results, got {len(results)})")
    return 0 if ok else 2


if __name__ == "__main__":
    raise SystemExit(main())
