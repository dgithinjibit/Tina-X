"""Project Nzi — harsh-weather nowcasting: synthetic data + probabilistic model (P5.1/P5.2).

DESIGN MANDATE (see weather-prediction/README.md): we target CALIBRATED PROBABILISTIC SKILL and
report honest scores (Brier, reliability, ETS, lead time). We NEVER ship a "99% accurate" claim —
the one 0.99 in the literature is pixel accuracy on one city/one variable with ETS only 0.18.

Because this environment has no internet/heavy ML deps and no real radar archive, we build a
SELF-CONTAINED, deterministic synthetic testbed that reproduces the STRUCTURE of the WoFS-style
task: an ensemble of noisy forecasts of a moving convective hazard, from which we predict a
calibrated hazard PROBABILITY per grid cell at a given lead time. This lets us build + measure the
symbolic fusion + calibration story now; real WoFS/radar data slots into the same interfaces later.

Pieces:
  * HazardField     — ground-truth: a moving "storm" blob over a grid at each time step.
  * ensemble()      — N noisy member forecasts of that field (stand-in for a WoFS ensemble).
  * ensemble_prob() — the baseline nowcast: fraction of members that flag a cell (P5.1 baseline).
  * swarm_prob()    — P5.2: in-situ swarm sensor readings sharpen the ensemble probability where
                      the fleet actually is (nodes contribute ground truth locally).
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass


@dataclass(frozen=True)
class HazardField:
    """Ground-truth hazard over a `size`×`size` grid: 1 where the storm exceeds threshold."""
    size: int
    cells: tuple[tuple[int, ...], ...]  # row-major 0/1

    def at(self, x: int, y: int) -> int:
        return self.cells[y][x]

    def positives(self) -> int:
        return sum(sum(row) for row in self.cells)


def _storm_center(t: int, size: int) -> tuple[float, float]:
    """A storm drifting diagonally across the grid over time (deterministic)."""
    frac = t / 10.0
    return (2.0 + frac * (size - 4), 2.0 + 0.6 * frac * (size - 4))


def truth_at(t: int, size: int, radius: float = 2.2) -> HazardField:
    """Ground-truth hazard field at time step `t`: a filled disk (the convective core)."""
    cx, cy = _storm_center(t, size)
    rows = []
    for y in range(size):
        row = []
        for x in range(size):
            hit = 1 if math.hypot(x - cx, y - cy) <= radius else 0
            row.append(hit)
        rows.append(tuple(row))
    return HazardField(size=size, cells=tuple(rows))


def ensemble(t: int, size: int, n_members: int, seed: int) -> list[HazardField]:
    """N noisy member forecasts of the hazard at time `t` (stand-in for a WoFS ensemble).

    Each member perturbs the storm center (track/timing uncertainty) and flips some cells (model
    error). Deterministic in `seed`. The SPREAD across members is what carries the probabilistic
    signal the nowcast extracts.
    """
    rng = random.Random(seed + t * 100003)
    members: list[HazardField] = []
    cx, cy = _storm_center(t, size)
    for _ in range(n_members):
        # Per-member track/timing perturbation.
        mx = cx + rng.gauss(0, 1.1)
        my = cy + rng.gauss(0, 1.1)
        rad = 2.2 + rng.gauss(0, 0.4)
        rows = []
        for y in range(size):
            row = []
            for x in range(size):
                hit = 1 if math.hypot(x - mx, y - my) <= rad else 0
                # Small independent model error (bit flips).
                if rng.random() < 0.03:
                    hit ^= 1
                row.append(hit)
            rows.append(tuple(row))
        members.append(HazardField(size=size, cells=tuple(rows)))
    return members


def ensemble_prob(members: list[HazardField]) -> list[list[float]]:
    """Baseline nowcast (P5.1): P(hazard) per cell = fraction of members flagging it.

    This is a genuinely probabilistic, reasonably-calibrated forecast — the honest baseline the
    symbolic layer (P5.3) and swarm fusion (P5.2) must IMPROVE on calibration/skill, not accuracy.
    """
    size = members[0].size
    n = len(members)
    return [[sum(m.at(x, y) for m in members) / n for x in range(size)] for y in range(size)]


def swarm_prob(
    base: list[list[float]],
    truth: HazardField,
    sensor_positions: list[tuple[int, int]],
    trust: float = 0.85,
) -> list[list[float]]:
    """P5.2 — swarm-as-sensor-fleet: sharpen the ensemble probability where nodes are physically
    present. A node at a cell contributes an in-situ reading (ground truth here), and we blend it
    toward that observation with weight `trust` (in-situ obs are more reliable than a forecast, but
    not perfect — sensors have noise, hence <1). Cells with no node keep the ensemble probability.

    This models the swarm's real value: it doesn't forecast better everywhere, it OBSERVES better
    where it is — a distributed sensor network improving the nowcast locally.
    """
    size = truth.size
    out = [row[:] for row in base]
    present = set(sensor_positions)
    for (x, y) in present:
        obs = float(truth.at(x, y))
        out[y][x] = trust * obs + (1.0 - trust) * base[y][x]
    return out
