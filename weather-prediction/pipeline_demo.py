#!/usr/bin/env python3
"""Project Nzi — weather nowcasting pipeline demo (P5.1/P5.2/P5.3/P5.5).

Ties the flagship edge case together, HONESTLY (Brier / reliability / ETS / lead time — never a
"99% accuracy" claim):

  1. baseline ensemble nowcast (P5.1),
  2. + swarm-as-sensor-fleet in-situ sharpening (P5.2),
  3. + MeTTa symbolic calibration/guidance (P5.3),
  scored with proper metrics at several lead times (P5.5).

The claim we make is modest and true: symbolic calibration IMPROVES reliability (lower Brier +
reliability penalty) over the raw ensemble, and the swarm improves skill where nodes observe.

Run:  python3 weather-prediction/pipeline_demo.py   (steps 1-2 need no hyperon; step 3 needs venv)
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from nowcast import ensemble, ensemble_prob, swarm_prob, truth_at  # noqa: E402
from verify import brier_score, ets, lead_time_curve, reliability  # noqa: E402

SIZE = 16
MEMBERS = 20
SEED = 7
# Lead times in minutes; the storm-step index t stands in for lead (6-min radar cadence).
LEADS = {6: 1, 30: 5, 60: 10}


def sensor_positions(size: int) -> list[tuple[int, int]]:
    """A swarm covering a diagonal swath of the field — where the fleet actually flies."""
    return [(i, i) for i in range(size)] + [(i, min(size - 1, i + 1)) for i in range(size)]


def score(prob, truth) -> dict:
    rel, _ = reliability(prob, truth)
    return {"brier": brier_score(prob, truth), "ets": ets(prob, truth), "reliability": rel}


def main() -> int:
    print("Project Nzi — harsh-weather nowcasting pipeline (Phase 5)")
    print("  metrics: Brier / reliability / ETS / lead time (NEVER a '99% accuracy' claim)")

    # Optional symbolic layer.
    try:
        from hyperon import MeTTa  # type: ignore
        from symbolic import apply_guidance  # local module
        metta = MeTTa()
        have_symbolic = True
    except ModuleNotFoundError:
        have_symbolic = False
        metta = None

    base_by_lead: dict[int, dict] = {}
    swarm_by_lead: dict[int, dict] = {}
    sym_by_lead: dict[int, dict] = {}

    positions = sensor_positions(SIZE)
    for lead_min, t in LEADS.items():
        truth = truth_at(t, SIZE)
        members = ensemble(t, SIZE, MEMBERS, SEED)
        base = ensemble_prob(members)
        base_by_lead[lead_min] = score(base, truth)

        swarm = swarm_prob(base, truth, positions)
        swarm_by_lead[lead_min] = score(swarm, truth)

        if have_symbolic:
            # Symbolic calibration applied to the swarm-fused probabilities (best input).
            prev = truth_at(t - 1, SIZE)
            calibrated = apply_guidance(swarm, members, prev, metta)
            sym_by_lead[lead_min] = score(calibrated, truth)

    print("\n  [P5.1] baseline ensemble nowcast:")
    print(lead_time_curve(base_by_lead))
    print("\n  [P5.2] + swarm-as-sensor-fleet (in-situ sharpening):")
    print(lead_time_curve(swarm_by_lead))
    if have_symbolic:
        print("\n  [P5.3] + MeTTa symbolic calibration/guidance:")
        print(lead_time_curve(sym_by_lead))

    # Honest verdict: symbolic calibration should reduce the reliability penalty vs. baseline.
    if have_symbolic:
        b0 = sum(s["reliability"] for s in base_by_lead.values())
        b1 = sum(s["reliability"] for s in sym_by_lead.values())
        improved = b1 <= b0
        print(f"\n  reliability penalty (sum over leads): baseline {b0:.5f} -> symbolic {b1:.5f}")
        print(f"  RESULT: {'OK — symbolic calibration improved reliability.' if improved else 'REVIEW — no calibration gain.'}")
        return 0 if improved else 2
    else:
        print("\n  (skipping symbolic step: hyperon not installed — activate the .venv)")
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
