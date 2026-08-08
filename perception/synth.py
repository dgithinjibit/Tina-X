"""Project Nzi — synthetic crop-imagery generator (P3.1).

WHY SYNTHETIC + STDLIB-ONLY:
  Real robotics perception is bottlenecked by labeled-data scarcity (see limitations-edge-cases/).
  Phase 3's build-here goal is the DECISION pipeline — perceive -> symbolic treat/don't-treat with
  justification — not a state-of-the-art detector. So we generate deterministic, labeled field
  patches procedurally (no downloads, no heavy deps, reproducible in CI). The real detector +
  Unity synthetic-data pipeline arrive later (P3.3 -> Phase 7); this pipeline is the honest,
  runnable stand-in that lets us build and MEASURE the treat-decision logic now.

A "patch" models a small overhead view of ground. It carries a few interpretable channels a cheap
on-board sensor could actually produce, not raw pixels:
  * green_frac  : fraction of vegetation cover (crop rows are greener/denser than bare soil),
  * row_align   : how well vegetation aligns to the planted crop rows (weeds grow off-row),
  * leaf_broad  : broadleaf texture score (many weeds are broadleaf vs. a narrow-leaf cereal crop),
  * ndvi_var    : local variance of a vegetation index (weed clumps are patchier than a crop stand).

Ground-truth label is one of: crop | weed | soil. These channels are enough for a simple, honest
classifier (see model.py) and map cleanly onto symbolic rules (see metta-logic/agronomy/).
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass


@dataclass(frozen=True)
class Patch:
    """One labeled ground patch: interpretable sensor channels + ground-truth class."""
    green_frac: float   # [0,1]
    row_align: float    # [0,1] — 1.0 = perfectly on crop rows
    leaf_broad: float   # [0,1] — 1.0 = strongly broadleaf
    ndvi_var: float     # [0,1] — patchiness of the vegetation index
    label: str          # "crop" | "weed" | "soil"

    def features(self) -> list[float]:
        """The feature vector the classifier sees (order is the model's contract)."""
        return [self.green_frac, self.row_align, self.leaf_broad, self.ndvi_var]


LABELS = ("crop", "weed", "soil")


def _clamp01(x: float) -> float:
    return 0.0 if x < 0.0 else 1.0 if x > 1.0 else x


def _sample(rng: random.Random, mean: float, sd: float) -> float:
    """A clamped Gaussian sample in [0,1] — keeps channels physically sensible."""
    return _clamp01(rng.gauss(mean, sd))


def make_patch(rng: random.Random, label: str) -> Patch:
    """Generate one patch for a given class, with class-characteristic channel distributions.

    The class means are deliberately overlapping (not linearly trivial) so precision/recall are
    meaningful — a perfect score would mean the synthetic task was too easy to be informative.
    """
    # Wider spreads + closer means so crop/weed genuinely OVERLAP: a trivially-perfect score would
    # mean the task is too easy to be informative. These distributions make precision/recall real.
    if label == "crop":
        # Dense green, mostly on-row, narrow-leaf, moderate patchiness.
        return Patch(
            green_frac=_sample(rng, 0.68, 0.16),
            row_align=_sample(rng, 0.72, 0.18),
            leaf_broad=_sample(rng, 0.35, 0.18),
            ndvi_var=_sample(rng, 0.35, 0.16),
            label=label,
        )
    if label == "weed":
        # Green but more OFF-row, broadleaf, patchier — overlaps crop on green_frac especially.
        return Patch(
            green_frac=_sample(rng, 0.60, 0.18),
            row_align=_sample(rng, 0.42, 0.20),
            leaf_broad=_sample(rng, 0.62, 0.18),
            ndvi_var=_sample(rng, 0.58, 0.18),
            label=label,
        )
    # soil: little vegetation; row/leaf/patchiness signals are weak/meaningless.
    return Patch(
        green_frac=_sample(rng, 0.18, 0.12),
        row_align=_sample(rng, 0.50, 0.22),
        leaf_broad=_sample(rng, 0.45, 0.22),
        ndvi_var=_sample(rng, 0.30, 0.18),
        label="soil",
    )


def make_dataset(n: int, seed: int, weights: tuple[float, float, float] = (0.5, 0.3, 0.2)) -> list[Patch]:
    """A deterministic labeled dataset of `n` patches (crop/weed/soil by `weights`).

    Deterministic in `seed` so training, evaluation, and tests are all reproducible — the same
    seed always yields the same field. `weights` reflects a realistic field: mostly crop, a
    meaningful weed minority, some bare soil.
    """
    assert abs(sum(weights) - 1.0) < 1e-6, "weights must sum to 1"
    rng = random.Random(seed)
    counts = [int(round(w * n)) for w in weights]
    # Fix rounding drift so len == n exactly.
    counts[0] += n - sum(counts)
    patches: list[Patch] = []
    for label, c in zip(LABELS, counts):
        for _ in range(c):
            patches.append(make_patch(rng, label))
    rng.shuffle(patches)
    return patches


def sigmoid(x: float) -> float:
    # Shared numeric helper (model.py imports it) — kept here so synth has no deps.
    if x < -60:
        return 0.0
    if x > 60:
        return 1.0
    return 1.0 / (1.0 + math.exp(-x))
