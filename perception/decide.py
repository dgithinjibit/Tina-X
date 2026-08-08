"""Project Nzi — perception -> symbolic treatment decision bridge (P3.2).

This is the sensor-fusion / decision seam: it takes a model Detection, hands it to the MeTTa
agronomy rules (metta-logic/agronomy/treat.metta), and returns a verifiable treat/skip verdict
WITH a symbolic justification. The ML model perceives; MeTTa decides and explains.

Mirrors the Rust brain bridge philosophy (ADR 0003): the numeric/perception side is Python, the
DECISION is symbolic and interpretable. A treatment (spraying pesticide) is irreversible and
costly, so — like the Phase-2 action gate — we only act on a confident, justified verdict.
"""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from model import Detection

HERE = Path(__file__).resolve().parent
AGRO_DIR = HERE.parent / "metta-logic" / "agronomy"


@dataclass(frozen=True)
class Verdict:
    """The symbolic treatment decision for one patch."""
    action: str   # "treat" | "skip"
    reason: str   # symbolic justification, e.g. "confident-weed" | "low-confidence-weed"

    @property
    def treat(self) -> bool:
        return self.action == "treat"


def agronomy_preamble() -> str:
    """Load the agronomy rule program (single source of truth on disk)."""
    return (AGRO_DIR / "treat.metta").read_text()


def _parse_verdict(atom: str) -> Verdict:
    """Parse MeTTa's `(treat <reason>)` / `(skip <reason>)` into a Verdict. Fail-closed: anything
    unrecognized becomes a skip, so an engine hiccup can never cause an unjustified spray."""
    a = atom.strip()
    for action in ("treat", "skip"):
        prefix = f"({action} "
        if a.startswith(prefix) and a.endswith(")"):
            return Verdict(action=action, reason=a[len(prefix):-1].strip())
    return Verdict(action="skip", reason="unparseable-verdict")


def decide(detection: Detection, metta=None) -> Verdict:
    """Run one Detection through the symbolic agronomy rules and return the Verdict.

    Pass a MeTTa instance to reuse one across many patches (cheaper); otherwise one is created.
    """
    if metta is None:
        from hyperon import MeTTa  # imported lazily so importing this module needs no hyperon
        metta = MeTTa()
    program = f"{agronomy_preamble()}\n!(decide-treatment {detection.as_metta_context()})"
    results = metta.run(program)[-1]
    if not results:
        return Verdict(action="skip", reason="no-result")
    return _parse_verdict(str(results[0]))
