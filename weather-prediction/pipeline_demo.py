#!/usr/bin/env python3
"""Project Nzi — weather nowcasting pipeline demo (P5.1/P5.2/P5.3/P5.4/P5.5).

Ties the flagship edge case together, HONESTLY (Brier / reliability / ETS / lead time — never a
"99% accuracy" claim):

  1. baseline ensemble nowcast                              (P5.1)
  2. + swarm-as-sensor-fleet in-situ sharpening             (P5.2)
  3. + thermodynamic pbit sampling (neighbor-local denoise) (P5.4, aspirational-but-real)
  4. + MeTTa symbolic calibration/guidance                  (P5.3)
  scored with proper metrics at several lead times          (P5.5)

The claims we make are modest and true: the swarm improves skill where nodes observe; neighbor-
coupled pbit sampling denoises per-cell speckle (lower Brier + reliability); symbolic calibration
sharpens by ensemble consensus and improves reliability. NO energy numbers are claimed for the TSU
step — it is the computation a TSU would run cheaply, verified here on a CPU.

Run:  python3 weather-prediction/pipeline_demo.py   (steps 1,2,3 need no hyperon; step 4 needs venv)
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from nowcast import ensemble, ensemble_prob, swarm_prob, truth_at  # noqa: E402
from gnss_pwv import PWV_DRY, PWV_MOIST, PwvStation, gnss_pwv_prob  # noqa: E402
from thermodynamic import thermodynamic_prob  # noqa: E402
from verify import brier_score, ets, lead_time_curve, reliability  # noqa: E402

# Demo grid. The MeTTa symbolic step (P5.3) runs one guidance atom per cell against a FRESH space
# per call (see symbolic.apply_guidance — reusing a space accumulates rules and blows up eval), so
# eval is now flat per lead. We use 12×12 here to keep the demo to ~30s total; the TEST SUITE runs
# 16×16 for metric fidelity. Either grid tells the same story: the stack improves calibration.
SIZE = 12
MEMBERS = 20
SEED = 7
# Lead times in minutes; the storm-step index t stands in for lead (6-min radar cadence).
LEADS = {6: 1, 30: 5, 60: 10}


def sensor_positions(size: int) -> list[tuple[int, int]]:
    """A swarm covering a diagonal swath of the field — where the fleet actually flies."""
    return [(i, i) for i in range(size)] + [(i, min(size - 1, i + 1)) for i in range(size)]


def pwv_stations(truth, size: int) -> list[PwvStation]:
    """A sparse net of ground GNSS stations reporting PWV (G4D-RR bridge #5). Synthetic feed: moist
    air over currently-hazardous cells, dry elsewhere — the plausible signal a real GNSS-met network
    would carry. Placed on a coarse lattice (every 3rd cell) like real, sparse CORS coverage."""
    stations = []
    for y in range(0, size, 3):
        for x in range(0, size, 3):
            pwv = PWV_MOIST + 5.0 if truth.at(x, y) == 1 else PWV_DRY - 5.0
            stations.append(PwvStation(x, y, pwv))
    return stations


def score(prob, truth) -> dict:
    rel, _ = reliability(prob, truth)
    return {"brier": brier_score(prob, truth), "ets": ets(prob, truth), "reliability": rel}


def say(*args) -> None:
    print(*args, flush=True)  # flush so the slow symbolic step shows progress, not a silent hang


def main() -> int:
    say("Project Nzi — harsh-weather nowcasting pipeline (Phase 5)")
    say("  metrics: Brier / reliability / ETS / lead time (NEVER a '99% accuracy' claim)")

    # Optional symbolic layer (P5.3). Steps 1-3 are pure stdlib and always run.
    try:
        import hyperon  # type: ignore  # noqa: F401
        from symbolic import apply_guidance  # local module (manages its own fresh MeTTa space)
        have_symbolic = True
    except ModuleNotFoundError:
        have_symbolic = False

    base_by_lead: dict[int, dict] = {}
    swarm_by_lead: dict[int, dict] = {}
    pwv_by_lead: dict[int, dict] = {}
    therm_by_lead: dict[int, dict] = {}
    sym_by_lead: dict[int, dict] = {}

    positions = sensor_positions(SIZE)
    for lead_min, t in LEADS.items():
        truth = truth_at(t, SIZE)
        members = ensemble(t, SIZE, MEMBERS, SEED)

        base = ensemble_prob(members)
        base_by_lead[lead_min] = score(base, truth)

        swarm = swarm_prob(base, truth, positions)
        swarm_by_lead[lead_min] = score(swarm, truth)

        # Bridge #5: GNSS-PWV ground stations add a moisture-availability observation (gentle nudge).
        pwv = gnss_pwv_prob(swarm, pwv_stations(truth, SIZE))
        pwv_by_lead[lead_min] = score(pwv, truth)

        # P5.4: neighbor-local pbit Gibbs sampling denoises the PWV-informed field.
        therm = thermodynamic_prob(pwv, seed=SEED)
        therm_by_lead[lead_min] = score(therm, truth)

        if have_symbolic:
            # P5.3: symbolic consensus-sharpening calibration on the denoised field (best input).
            prev = truth_at(t - 1, SIZE)
            say(f"  ... symbolic calibration for lead {lead_min} min ({SIZE*SIZE} cells)")
            calibrated = apply_guidance(therm, members, prev)
            sym_by_lead[lead_min] = score(calibrated, truth)

    say("\n  [P5.1] baseline ensemble nowcast:")
    say(lead_time_curve(base_by_lead))
    say("\n  [P5.2] + swarm-as-sensor-fleet (in-situ sharpening):")
    say(lead_time_curve(swarm_by_lead))
    say("\n  [G4D-RR #5] + GNSS-PWV ground-station moisture observation:")
    say(lead_time_curve(pwv_by_lead))
    say("\n  [P5.4] + thermodynamic pbit sampling (neighbor-local denoise):")
    say(lead_time_curve(therm_by_lead))
    if have_symbolic:
        say("\n  [P5.3] + MeTTa symbolic calibration/guidance:")
        say(lead_time_curve(sym_by_lead))

    # Honest verdict: the reliability penalty should fall from raw ensemble to the full stack.
    say("\n  reliability penalty (sum over leads, lower = better calibrated):")
    r_base = sum(s["reliability"] for s in base_by_lead.values())
    r_therm = sum(s["reliability"] for s in therm_by_lead.values())
    say(f"    baseline {r_base:.5f}  ->  + thermodynamic {r_therm:.5f}")
    if have_symbolic:
        r_sym = sum(s["reliability"] for s in sym_by_lead.values())
        say(f"    + symbolic {r_sym:.5f}")
        improved = r_sym <= r_base
        say(f"  RESULT: {'OK — the stack improved calibration over the raw ensemble.' if improved else 'REVIEW — no calibration gain.'}")
        return 0 if improved else 2
    else:
        improved = r_therm <= r_base
        say(f"  RESULT: {'OK — thermodynamic denoising improved calibration.' if improved else 'REVIEW.'}")
        say("  (symbolic step skipped: hyperon not installed — activate the .venv for P5.3)")
        return 0 if improved else 2


if __name__ == "__main__":
    raise SystemExit(main())
