#!/usr/bin/env python3
"""Project Nzi — supervisory knowledge-base smoke test (P2.4).

Loads the knowledge base (world + mission + agent) and asserts that `decide-setpoint` proposes a
sensible, in-envelope setpoint for representative telemetry:
  * in `free` mode it proposes the mission's modest forward target, and
  * in `hold-level` mode it proposes zero roll/pitch/yaw (the invariant the memorization check
    later enforces) — so the brain's own proposal is consistent with the gate.

This proves the KB reasoning works BEFORE the Rust supervisor (P2.5) wires it to the gate. It uses
the SAME load order the Rust supervisor uses.

Run:  python3 metta-logic/run_supervise_smoke.py   (needs the .venv with hyperon)
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
KDIR = HERE / "knowledge"

# world defines helpers/limits used by mission; agent is standalone. Load world -> mission -> agent.
LOAD_ORDER = ["world.metta", "mission.metta", "agent.metta"]


def preamble() -> str:
    return "\n".join((KDIR / name).read_text() for name in LOAD_ORDER)


# (label, telemetry term, expected proposed setpoint)
CASES = [
    # MeTTa normalizes 0.0 -> 0 in output; the roll/yaw targets carry their decimals.
    ("free-mode-proposes-target",
     "(telemetry (measured (rates 0.4 0.0 0.1)) (mode free))",
     "(setpoint (rates 0.5 0 0.2))"),
    ("hold-level-proposes-zero",
     "(telemetry (measured (rates 0.0 0.0 0.0)) (mode hold-level))",
     "(setpoint (rates 0 0 0))"),
]


def main() -> int:
    try:
        from hyperon import MeTTa  # type: ignore
    except ModuleNotFoundError:
        print("ERROR: hyperon not installed (activate the .venv).", file=sys.stderr)
        return 1

    base = preamble()
    print("Project Nzi — supervisory knowledge-base smoke test")
    failures = 0

    for label, telemetry, expected in CASES:
        metta = MeTTa()
        query = f"{base}\n!(decide-setpoint {telemetry})"
        results = metta.run(query)[-1]
        got = str(results[0]) if results else "(no result)"
        ok = got == expected
        failures += 0 if ok else 1
        print(f"  [{'OK ' if ok else 'BAD'}] {label:28s} -> {got}"
              + ("" if ok else f"   (expected {expected})"))

    print(f"  RESULT: {'ALL PASS' if failures == 0 else f'{failures} FAILED'}")
    return 0 if failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
