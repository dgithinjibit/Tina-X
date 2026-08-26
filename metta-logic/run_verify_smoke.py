#!/usr/bin/env python3
"""Project TINA-X — verification-moat smoke test (P2.1).

Loads the five hallucination-type checks + the gate composer, then asserts that:
  * a well-formed, safe setpoint is Approved, and
  * a representative BAD setpoint for EACH of the five types is Rejected with the right reason.

This is the standalone proof that the symbolic verification layer works before the Rust
action-gate (P2.2) wires it into the agent. It uses the SAME concatenation order the Rust gate
uses, so if this passes, the gate is talking to correct rules.

Run:  python3 metta-logic/run_verify_smoke.py   (needs the .venv with hyperon)
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
VDIR = HERE / "verification"

# The load order the Rust gate also uses: shared limits, the five checks, then the composer.
# (verify.metta references the check predicates, so it must come last.)
LOAD_ORDER = [
    "limits.metta",
    "perception.metta",
    "reasoning.metta",
    "execution.metta",
    "memorization.metta",
    "communication.metta",
    "verify.metta",
]


def preamble() -> str:
    """Concatenate the verification program in dependency order."""
    return "\n".join((VDIR / name).read_text() for name in LOAD_ORDER)


# A base context that passes every check; each test tweaks exactly one thing to trip one check.
GOOD_CTX = "(context (measured (rates 0.4 0.0 0.1)) True True local-brain free)"
GOOD_ACTION = "(setpoint (rates 0.5 0.0 0.2))"

# (label, action, context, expected verdict string)
CASES = [
    ("approved-good", GOOD_ACTION, GOOD_CTX, "Approved"),
    # Perception: telemetry not fresh (3rd field False).
    ("reject-perception", GOOD_ACTION,
     "(context (measured (rates 0.4 0.0 0.1)) False True local-brain free)",
     "(Rejected perception)"),
    # Communication: unknown source.
    ("reject-communication", GOOD_ACTION,
     "(context (measured (rates 0.4 0.0 0.1)) True True unknown free)",
     "(Rejected communication)"),
    # Reasoning: 9.0 rad/s exceeds max-rate (3.0).
    ("reject-reasoning", "(setpoint (rates 9.0 0.0 0.2))", GOOD_CTX,
     "(Rejected reasoning)"),
    # Execution: 0.4 -> 2.5 is a 2.1 step (> max-rate-step 1.5) but 2.5 < max-rate so reasoning passes.
    ("reject-execution", "(setpoint (rates 2.5 0.0 0.2))", GOOD_CTX,
     "(Rejected execution)"),
    # Memorization: hold-level mode requires zero roll/pitch, but roll is 0.5.
    ("reject-memorization", GOOD_ACTION,
     "(context (measured (rates 0.4 0.0 0.1)) True True local-brain hold-level)",
     "(Rejected memorization)"),
]


def main() -> int:
    try:
        from hyperon import MeTTa  # type: ignore
    except ModuleNotFoundError:
        print("ERROR: hyperon not installed (activate the .venv).", file=sys.stderr)
        return 1

    base = preamble()
    print("Project TINA-X — verification-moat smoke test")
    failures = 0

    for label, action, ctx, expected in CASES:
        # Fresh interpreter per case so no state leaks between queries.
        metta = MeTTa()
        query = f"{base}\n!(gate-setpoint {action} {ctx})"
        results = metta.run(query)[-1]
        got = str(results[0]) if results else "(no result)"
        ok = got == expected
        failures += 0 if ok else 1
        print(f"  [{'OK ' if ok else 'BAD'}] {label:22s} -> {got}"
              + ("" if ok else f"   (expected {expected})"))

    print(f"  RESULT: {'ALL PASS' if failures == 0 else f'{failures} FAILED'}")
    return 0 if failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
