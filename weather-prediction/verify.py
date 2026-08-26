"""Project TINA-X — honest nowcast verification metrics (P5.5), stdlib only.

The metrics we ACTUALLY report (never a headline "accuracy"):
  * Brier score          — mean squared error of probabilistic forecasts; lower = better. The
                           single most honest scalar for a probability forecast.
  * Reliability diagram  — binned forecast-probability vs. observed frequency; the diagonal is
                           perfect calibration. We summarize it as a scalar RELIABILITY term
                           (weighted squared gap from the diagonal; lower = better calibrated).
  * ETS (Gilbert Skill)  — Equitable Threat Score at a decision threshold; skill over random,
                           the operationally serious metric (WoFS reports this, not accuracy).
  * Lead-time skill      — how ETS/Brier degrade as forecast lead time grows (the practically
                           important question: how early can we warn?).

Why these and not accuracy: severe weather is RARE, so a model that always says "no hazard" scores
~99% accuracy while being useless. Brier/ETS/reliability are not fooled by class imbalance.
"""
from __future__ import annotations


def _flatten(prob: list[list[float]], truth) -> list[tuple[float, int]]:
    size = truth.size
    return [(prob[y][x], truth.at(x, y)) for y in range(size) for x in range(size)]


def brier_score(prob: list[list[float]], truth) -> float:
    """Mean of (p - o)^2 over all cells. 0 = perfect, 0.25 = always-0.5, up to 1 = confidently wrong."""
    pairs = _flatten(prob, truth)
    return sum((p - o) ** 2 for p, o in pairs) / len(pairs)


def reliability(prob: list[list[float]], truth, bins: int = 10) -> tuple[float, list[dict]]:
    """Reliability diagram + a scalar calibration penalty.

    Returns (reliability_penalty, diagram). The penalty is the frequency-weighted mean squared gap
    between forecast probability and observed frequency per bin — 0 means perfectly calibrated. The
    diagram is a list of {bin_mid, forecast_mean, observed_freq, count} for plotting/printing.
    """
    pairs = _flatten(prob, truth)
    buckets: list[list[tuple[float, int]]] = [[] for _ in range(bins)]
    for p, o in pairs:
        idx = min(bins - 1, int(p * bins))
        buckets[idx].append((p, o))

    diagram: list[dict] = []
    penalty = 0.0
    total = len(pairs)
    for b, bucket in enumerate(buckets):
        if not bucket:
            diagram.append({"bin_mid": (b + 0.5) / bins, "forecast_mean": None,
                            "observed_freq": None, "count": 0})
            continue
        fmean = sum(p for p, _ in bucket) / len(bucket)
        ofreq = sum(o for _, o in bucket) / len(bucket)
        penalty += (len(bucket) / total) * (fmean - ofreq) ** 2
        diagram.append({"bin_mid": (b + 0.5) / bins, "forecast_mean": fmean,
                        "observed_freq": ofreq, "count": len(bucket)})
    return penalty, diagram


def ets(prob: list[list[float]], truth, threshold: float = 0.5) -> float:
    """Equitable Threat Score (Gilbert Skill Score) at a probability `threshold`.

    ETS = (hits - hits_random) / (hits + misses + false_alarms - hits_random), where
    hits_random = (hits+misses)(hits+false_alarms)/total. Range (-1/3, 1]; 0 = no skill over random,
    1 = perfect. This is the metric that is NOT fooled by rare events.
    """
    pairs = _flatten(prob, truth)
    total = len(pairs)
    hits = misses = false_alarms = 0
    for p, o in pairs:
        pred = 1 if p >= threshold else 0
        if pred == 1 and o == 1:
            hits += 1
        elif pred == 0 and o == 1:
            misses += 1
        elif pred == 1 and o == 0:
            false_alarms += 1
    hits_random = (hits + misses) * (hits + false_alarms) / total if total else 0.0
    denom = hits + misses + false_alarms - hits_random
    return (hits - hits_random) / denom if denom != 0 else 0.0


def lead_time_curve(scores_by_lead: dict[int, dict]) -> str:
    """Format a lead-time skill table: for each lead (minutes), its Brier + ETS. The honest answer
    to 'how early can we warn?' — skill should degrade gracefully, not cliff."""
    lines = ["  lead(min)  Brier   ETS"]
    for lead in sorted(scores_by_lead):
        s = scores_by_lead[lead]
        lines.append(f"  {lead:>7}   {s['brier']:.4f}  {s['ets']:.3f}")
    return "\n".join(lines)
