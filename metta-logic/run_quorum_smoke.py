#!/usr/bin/env python3
"""Project TINA-X — bee quorum + cross-inhibition smoke test (bee brain upgrade B1-B4).

Loads metta-logic/knowledge/quorum.metta and asserts the collective-decision rule behaves:
  * below quorum -> Scout (no blind commit — fail-safe);
  * best-supported option past quorum -> Commit to it;
  * B2 risk knob: a HIGH-risk context needs far more support than a LOW-risk one;
  * B3/B4 cross-inhibition: at LOW quorum, rival stop-signals flip which option wins (and can hold
    a contested leader below quorum), while at HIGH quorum inhibition is OFF (raw support decides).

Run:  python3 metta-logic/run_quorum_smoke.py   (needs the .venv with hyperon)
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
QUORUM = HERE / "knowledge" / "quorum.metta"


def preamble() -> str:
    return QUORUM.read_text()


# A MeTTa cons-list of options: (:: o1 (:: o2 ... ())). We pass the first option and the REST list
# to decide-quorum, matching its (ctx first rest) signature.
def opt(id_: str, support: int, inhibition: int) -> str:
    # FLAT positional term — no (id ..)/(support ..) sub-wrappers (they'd be evaluated away).
    return f"(opt {id_} {support} {inhibition})"


def cons_list(items: list[str]) -> str:
    out = "()"
    for it in reversed(items):
        out = f"(:: {it} {out})"
    return out


# (label, risk, first-option, rest-options, EXACT expected reduced verdict). We assert the EXACT
# reduced atom — a substring check would falsely "pass" on an unreduced (if ...) blob (the MeTTa
# gotcha that bit the first version of this file).
CASES = [
    # Low-risk quorum is 6. A lone option with support 7 (no inhibition) commits with weight 7.
    ("low-commit", "low", opt("north", 7, 0), [], "(Commit north 7 quorum-reached)"),
    # Low-risk, support 5 < quorum 6 -> keep scouting.
    ("low-below-quorum", "low", opt("north", 5, 0), [], "(Scout below-quorum)"),
    # High-risk quorum is 20. Support 7 is plenty for low-risk but NOT for high -> Scout.
    ("high-needs-more", "high", opt("north", 7, 0), [], "(Scout below-quorum)"),
    # High-risk, support 21 -> Commit (inhibition is OFF at high risk, so raw support rules).
    ("high-commit", "high", opt("north", 21, 0), [], "(Commit north 21 quorum-reached)"),
    # B3 at LOW quorum: two options; 'south' raw support 8 but received 5 stop-signals -> effective
    # 3; 'north' support 7, no inhibition -> effective 7 wins AND is past quorum 6 -> commit north@7.
    ("low-inhibition-flips-winner", "low", opt("north", 7, 0), [opt("south", 8, 5)],
     "(Commit north 7 quorum-reached)"),
    # B4 at HIGH quorum: inhibition OFF, so 'south' (raw 22) beats 'north' (raw 7) and, past quorum
    # 20, commits to south@22 — proving inhibition does NOT subtract at high risk (gain 0).
    ("high-inhibition-off", "high", opt("north", 7, 9), [opt("south", 22, 9)],
     "(Commit south 22 quorum-reached)"),
]


def main() -> int:
    try:
        from hyperon import MeTTa  # type: ignore
    except ModuleNotFoundError:
        print("ERROR: hyperon not installed (activate the .venv).", file=sys.stderr)
        return 1

    base = preamble()
    print("Project TINA-X — bee quorum + cross-inhibition smoke test")
    failures = 0

    for label, risk, first, rest, expected in CASES:
        metta = MeTTa()  # fresh space per case (never reuse — see tina-metta-gotchas #6)
        ctx = f"(qctx (risk {risk}))"
        query = f"{base}\n!(decide-quorum {ctx} {first} {cons_list(rest)})"
        results = metta.run(query)[-1]
        got = str(results[0]) if results else "(no result)"
        ok = got == expected  # EXACT reduced-atom match — never a substring (see the gotcha above)
        failures += 0 if ok else 1
        print(f"  [{'OK ' if ok else 'BAD'}] {label:28s} -> {got}"
              + ("" if ok else f"   (expected {expected})"))

    print(f"  RESULT: {'ALL PASS' if failures == 0 else f'{failures} FAILED'}")
    return 0 if failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
