"""Project Nzi — bridge from ensemble nowcast to the MeTTa hazard-guidance layer (P5.3).

Runs each cell's (probability, ensemble-agreement, persistence) through the symbolic constraint
rules in metta-logic/weather/hazard.metta and returns the CALIBRATED probability field. The
symbolic layer's calibration is what improves reliability (measured in verify.py) — the numeric
side proposes, the symbolic side makes it interpretable and better-calibrated.
"""
from __future__ import annotations

from pathlib import Path

HERE = Path(__file__).resolve().parent
WEATHER_METTA = HERE.parent / "metta-logic" / "weather" / "hazard.metta"


def guidance_preamble() -> str:
    return WEATHER_METTA.read_text()


def _members_agree(members, x: int, y: int) -> float:
    """Fraction of ensemble members flagging this cell — the agreement/confidence proxy."""
    n = len(members)
    return sum(m.at(x, y) for m in members) / n if n else 0.0


def _parse_prob(atom: str) -> float | None:
    """Pull the calibrated probability out of `(guidance (level L) (prob P) (reason R))`."""
    marker = "(prob "
    i = atom.find(marker)
    if i < 0:
        return None
    j = atom.find(")", i)
    try:
        return float(atom[i + len(marker):j].strip())
    except ValueError:
        return None


def apply_guidance(prob, members, prev_truth, metta) -> list[list[float]]:
    """Return a new probability field with the MeTTa symbolic calibration applied per cell.

    Batches all cells into ONE MeTTa program (fast) and reads back the calibrated prob per cell.
    Falls back to the input probability for any cell whose verdict fails to parse (fail-safe: we
    never fabricate a hazard number we didn't get from the rules).
    """
    size = len(prob)
    preamble = guidance_preamble()

    # Build one program: the rules, then one !(hazard-guidance ...) per cell, in row-major order.
    lines = [preamble]
    for y in range(size):
        for x in range(size):
            p = prob[y][x]
            agree = _members_agree(members, x, y)
            persist = float(prev_truth.at(x, y))
            lines.append(
                f"!(hazard-guidance (nowcast (prob {p:.4f}) "
                f"(members-agree {agree:.4f}) (persistence {persist:.1f})))"
            )
    program = "\n".join(lines)

    results = metta.run(program)
    # metta.run returns one result-list per '!' line, in order. Map them back to the grid.
    out = [row[:] for row in prob]
    flat = [r for r in results if r]  # keep non-empty result lists, in order
    idx = 0
    for y in range(size):
        for x in range(size):
            if idx < len(flat) and flat[idx]:
                cal = _parse_prob(str(flat[idx][0]))
                if cal is not None:
                    out[y][x] = cal
            idx += 1
    return out
