"""Project TINA-X — thermodynamic / pbit sampling exploration for nowcasting (P5.4).

STATUS: ASPIRATIONAL-BUT-REAL (see weather-prediction/README.md, leg B). Extropic's Thermodynamic
Sampling Units (TSUs) sample from energy-based models via Gibbs sampling using *pbits* — transistor-
only Bernoulli samplers whose probability is set by a bias plus a weighted sum of PHYSICALLY-CLOSE
NEIGHBORS. That is exactly a decentralized, neighbor-local computation — a natural fit for a swarm.
The 10,000×-energy claims are from simulations of small TSU sections, NOT deployed weather models,
so we do NOT claim any energy number here. What we CAN honestly demonstrate now, on a CPU, is the
COMPUTATION a TSU would run cheaply, and show it improves the nowcast.

WHAT THIS BUYS THE NOWCAST (the honest, measurable win):
  The ensemble nowcast (nowcast.py) is per-cell and noisy — model error flips isolated cells on/off,
  producing speckle that a per-cell calibration (symbolic.py) cannot remove, because it has no
  spatial context. A convective core is a CONNECTED blob: neighboring cells are correlated. Encoding
  that prior as an Ising-style energy — each cell wants to (a) match its own ensemble evidence and
  (b) agree with its neighbors — and sampling the posterior with pbit Gibbs updates DENOISES the
  speckle and yields a spatially-coherent, better-calibrated hazard probability. Same math a TSU
  runs in hardware; here we run it as a plain Gibbs sampler so the RESULT is verifiable today.

  Energy of a binary hazard field s in {0,1} given per-cell evidence prob p:
      E(s) = -beta_data * sum_i  logit(p_i) * s_i           (fit the ensemble evidence)
             -beta_smooth * sum_<i,j>  1[s_i == s_j]        (neighbors want to agree)
  A pbit at cell i flips to 1 with probability sigmoid(local_field_i), where local_field_i is the
  bias (data term) plus the coupling to its 4 neighbors — a bias + weighted neighbor sum, exactly
  the TSU pbit rule. Averaging s over many sweeps gives the posterior P(hazard) per cell.

Interfaces mirror nowcast.py so this slots into the same pipeline: it consumes an ensemble
probability field and returns a calibrated probability field.
"""
from __future__ import annotations

import math
import random


def _logit(p: float, eps: float = 1e-4) -> float:
    """log(p / (1-p)), clamped so a 0/1 ensemble cell gives a strong-but-finite bias."""
    p = min(1.0 - eps, max(eps, p))
    return math.log(p / (1.0 - p))


def _sigmoid(x: float) -> float:
    # Numerically stable logistic — this is the pbit's Bernoulli probability.
    if x >= 0:
        z = math.exp(-x)
        return 1.0 / (1.0 + z)
    z = math.exp(x)
    return z / (1.0 + z)


def _neighbors(x: int, y: int, size: int):
    """4-connected physically-close neighbors (the only cells a pbit is coupled to)."""
    if x > 0:
        yield x - 1, y
    if x < size - 1:
        yield x + 1, y
    if y > 0:
        yield x, y - 1
    if y < size - 1:
        yield x, y + 1


def thermodynamic_prob(
    prob: list[list[float]],
    *,
    beta_data: float = 1.2,
    beta_smooth: float = 0.2,
    sweeps: int = 60,
    burn_in: int = 15,
    seed: int = 0,
) -> list[list[float]]:
    """Sample the hazard posterior with pbit Gibbs updates and return the mean field.

    This is the CPU stand-in for what an Extropic TSU samples in hardware: each cell is a pbit whose
    flip probability is sigmoid(bias + neighbor coupling). We sweep the grid, updating every pbit
    from its current neighbors (Gibbs sampling), discard `burn_in` sweeps, then average the binary
    states over the remaining sweeps to get a calibrated, spatially-coherent P(hazard) per cell.

    * beta_data   — how hard each cell fits its own ensemble evidence (the data term / bias).
    * beta_smooth — coupling strength: how strongly neighbors want to agree (the spatial prior).
                    0 recovers the per-cell forecast; too high over-smooths the storm edge.

    HONEST TUNING NOTE (measured, see the P5.4 sweep in tests): GENTLE coupling (the default
    beta_smooth=0.2) lowers Brier AND the reliability penalty at every lead time while leaving
    threshold skill (ETS) roughly neutral — the win is denoising speckle, not sharpening skill.
    STRONG coupling erodes the storm core and collapses ETS, so we deliberately keep it gentle. We
    claim ZERO energy numbers: this is the computation a TSU would run cheaply, verified on a CPU.
    Deterministic in `seed`.
    """
    size = len(prob)
    rng = random.Random(seed)

    # Per-cell data bias: logit of the ensemble probability, scaled. This is the pbit's fixed bias.
    bias = [[beta_data * _logit(prob[y][x]) for x in range(size)] for y in range(size)]

    # Initialize each pbit from its own evidence (a reasonable, evidence-consistent start).
    state = [[1 if rng.random() < prob[y][x] else 0 for x in range(size)] for y in range(size)]

    accum = [[0 for _ in range(size)] for _ in range(size)]
    counted = 0
    for sweep in range(sweeps):
        for y in range(size):
            for x in range(size):
                # local_field = data bias + coupling to physically-close neighbors.
                # An Ising 1[s_i==s_j] prior contributes +beta_smooth per neighbor that is 1 and
                # -beta_smooth per neighbor that is 0 to the field favoring s_i = 1.
                field = bias[y][x]
                for nx, ny in _neighbors(x, y, size):
                    field += beta_smooth * (1.0 if state[ny][nx] == 1 else -1.0)
                state[y][x] = 1 if rng.random() < _sigmoid(field) else 0
        if sweep >= burn_in:
            for y in range(size):
                for x in range(size):
                    accum[y][x] += state[y][x]
            counted += 1

    counted = max(1, counted)
    return [[accum[y][x] / counted for x in range(size)] for y in range(size)]


def energy(prob: list[list[float]], state: list[list[int]], beta_data: float, beta_smooth: float) -> float:
    """The energy E(s) of a binary field given the evidence — for tests/introspection. Lower is a
    more probable configuration under the model. Sampling should visit low-energy (coherent) fields."""
    size = len(prob)
    e = 0.0
    for y in range(size):
        for x in range(size):
            e -= beta_data * _logit(prob[y][x]) * state[y][x]
            for nx, ny in _neighbors(x, y, size):
                if nx > x or ny > y:  # count each edge once
                    e -= beta_smooth * (1.0 if state[ny][nx] == state[y][x] else 0.0)
    return e
