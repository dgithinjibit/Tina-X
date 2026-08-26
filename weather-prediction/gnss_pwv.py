"""Project TINA-X — GNSS-derived Precipitable Water Vapor as a nowcasting input (G4D-RR bridge #5).

See docs/research/g4drr-gnss-eo-bridge.md. A ground GNSS receiver's signal delay yields the
Zenith Tropospheric Delay, which converts to **Precipitable Water Vapor (PWV)** — the column of
water available to a storm. PWV is a genuine, cheap, all-weather OBSERVATION (RINEX -> a PPP tool
like MG-APP -> PWV, from a single station), and rising PWV precedes convection, so it sharpens a
short-range hazard nowcast where stations are.

This mirrors the swarm-as-sensor model (`nowcast.swarm_prob`): a per-station in-situ signal nudges
the ensemble probability LOCALLY. The difference is what the sensor measures — PWV is a *proxy*
(more moisture -> more hazard potential), not ground truth, so its nudge is gentle and, crucially,
one-directional-ish: high PWV can only RAISE suspicion.

HONESTY GUARDRAIL (research caveat, do not violate): GNSS-PWV **raises the false-alarm rate** and is
threshold-dependent. It is not a free accuracy win. We therefore:
  * blend gently (small weight) and never hard-set a cell to 1.0 from PWV alone;
  * expose the effect so it is judged by the SAME honest metrics as everything else (Brier,
    reliability, ETS in verify.py) — a bad threshold will SHOW UP as worse calibration, not be hidden.
This module never claims a "99%"-style number; it adds a real observation and lets the metrics rule.
"""
from __future__ import annotations

from dataclasses import dataclass


# A conventional convective-relevant PWV scale (mm). Values well below `PWV_DRY` carry little hazard
# signal; values at/above `PWV_MOIST` indicate a moisture-laden column that can feed convection.
# These are illustrative thresholds for the synthetic testbed, tunable per climate — NOT universal
# physics. The point is the pipeline + honest measurement, not calibrated hydrometeorology.
PWV_DRY = 30.0
PWV_MOIST = 55.0


@dataclass(frozen=True)
class PwvStation:
    """One GNSS ground station reporting PWV at a grid cell."""
    x: int
    y: int
    pwv_mm: float


def pwv_to_signal(pwv_mm: float) -> float:
    """Map a PWV reading (mm) to a hazard-suspicion signal in [0,1] by linear ramp between
    `PWV_DRY` (0) and `PWV_MOIST` (1), clamped. This is the moisture-availability proxy; it is
    deliberately monotone and bounded so it can only add bounded suspicion, never certainty."""
    if PWV_MOIST <= PWV_DRY:
        return 0.0
    return max(0.0, min(1.0, (pwv_mm - PWV_DRY) / (PWV_MOIST - PWV_DRY)))


def gnss_pwv_prob(
    base: list[list[float]],
    stations: list[PwvStation],
    weight: float = 0.25,
) -> list[list[float]]:
    """Nudge the ensemble probability upward at cells where a GNSS station reports moist air.

    For a station cell: `out = base + weight * signal * (1 - base)` — a gentle move of the remaining
    headroom toward 1, scaled by the PWV signal and a small `weight`. Properties, all intentional:
      * PWV can only RAISE probability (moisture is a hazard *ingredient*, never a suppressor here);
      * the result stays in [0,1] and never reaches 1.0 from PWV alone (weight<1, headroom factor);
      * cells with no station are unchanged.
    Keep `weight` small: the research warns PWV inflates false alarms, and verify.py's Brier/
    reliability will penalize an over-eager weight — so this is honest, measurable, and tunable.
    """
    out = [row[:] for row in base]
    w = max(0.0, min(1.0, weight))
    for st in stations:
        if 0 <= st.y < len(out) and 0 <= st.x < len(out[0]):
            signal = pwv_to_signal(st.pwv_mm)
            b = out[st.y][st.x]
            out[st.y][st.x] = b + w * signal * (1.0 - b)
    return out
