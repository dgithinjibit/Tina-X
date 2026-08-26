"""Project TINA-X — weed/pest detection model (P3.1) + honest metrics (P3.4), stdlib only.

A small multiclass logistic-regression classifier (one-vs-rest, trained by batch gradient descent)
over the interpretable channels from synth.py. No numpy/sklearn — pure Python — so it runs anywhere
the repo does, deterministically, without downloads.

This is intentionally simple. The POINT of Phase 3 (build-here) is the perceive -> symbolic
treat/don't-treat -> justify pipeline and its MEASURED quality, not a SOTA detector. The model
emits a calibrated per-class probability + confidence, which is exactly what the symbolic layer
(metta-logic/agronomy/) needs to make a verifiable, confidence-gated treatment decision.
"""
from __future__ import annotations

from dataclasses import dataclass, field

from synth import LABELS, Patch, sigmoid


@dataclass
class Detection:
    """The model's output for one patch — the perception half of the treat decision."""
    label: str                       # argmax predicted class
    probs: dict[str, float]          # per-class probability (normalized)
    confidence: float                # probability of the winning class

    def as_metta_context(self) -> str:
        """Render as the MeTTa perception term the agronomy rules consume:
            (detection (class <label>) (confidence <c>))
        Confidence is rounded so the symbolic side compares stable literals."""
        return f"(detection (class {self.label}) (confidence {self.confidence:.3f}))"


@dataclass
class LogisticModel:
    """One-vs-rest logistic regression. `w[c]` are the weights for class c, `b[c]` its bias."""
    n_features: int
    w: dict[str, list[float]] = field(default_factory=dict)
    b: dict[str, float] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not self.w:
            self.w = {c: [0.0] * self.n_features for c in LABELS}
            self.b = {c: 0.0 for c in LABELS}

    def _raw(self, c: str, x: list[float]) -> float:
        return sigmoid(sum(wi * xi for wi, xi in zip(self.w[c], x)) + self.b[c])

    def predict(self, x: list[float]) -> Detection:
        """Per-class one-vs-rest scores, normalized into a probability distribution."""
        scores = {c: self._raw(c, x) for c in LABELS}
        total = sum(scores.values()) or 1.0
        probs = {c: s / total for c, s in scores.items()}
        label = max(probs, key=probs.get)
        return Detection(label=label, probs=probs, confidence=probs[label])


def train(patches: list[Patch], epochs: int = 300, lr: float = 0.5) -> LogisticModel:
    """Train one-vs-rest logistic regression by batch gradient descent. Deterministic (no RNG)."""
    model = LogisticModel(n_features=len(patches[0].features()))
    n = len(patches)
    xs = [p.features() for p in patches]
    for c in LABELS:
        ys = [1.0 if p.label == c else 0.0 for p in patches]
        for _ in range(epochs):
            grad_w = [0.0] * model.n_features
            grad_b = 0.0
            for x, y in zip(xs, ys):
                err = model._raw(c, x) - y
                for j in range(model.n_features):
                    grad_w[j] += err * x[j]
                grad_b += err
            for j in range(model.n_features):
                model.w[c][j] -= lr * grad_w[j] / n
            model.b[c] -= lr * grad_b / n
    return model


@dataclass
class Metrics:
    """Honest per-class precision/recall/F1 + confusion, plus the pesticide-reduction story (P3.4)."""
    precision: dict[str, float]
    recall: dict[str, float]
    f1: dict[str, float]
    confusion: dict[str, dict[str, str]]  # true -> {pred -> count} kept as str for tidy printing
    accuracy: float
    pesticide_reduction_pct: float
    blanket_treatments: int
    targeted_treatments: int

    def summary(self) -> str:
        lines = ["Perception metrics (synthetic eval):"]
        for c in LABELS:
            lines.append(
                f"  {c:5s}  P={self.precision[c]:.3f}  R={self.recall[c]:.3f}  F1={self.f1[c]:.3f}"
            )
        lines.append(f"  accuracy = {self.accuracy:.3f}")
        lines.append(
            f"  pesticide reduction = {self.pesticide_reduction_pct:.1f}% "
            f"(blanket {self.blanket_treatments} -> targeted {self.targeted_treatments} patches)"
        )
        return "\n".join(lines)


def evaluate(model: LogisticModel, patches: list[Patch]) -> Metrics:
    """Compute precision/recall/F1 + the pesticide-reduction metric on a held-out set.

    Pesticide reduction (the economic story): the naive baseline is a BLANKET spray — treat every
    patch. Our targeted policy sprays only patches the model calls `weed`. Reduction is the
    fraction of blanket sprays we avoid. (This counts targeted-by-detection; the symbolic layer
    then further gates it by confidence — see decide.py — which can only REDUCE spraying more.)
    """
    tp = {c: 0 for c in LABELS}
    fp = {c: 0 for c in LABELS}
    fn = {c: 0 for c in LABELS}
    confusion = {t: {p: 0 for p in LABELS} for t in LABELS}
    correct = 0

    targeted = 0
    for patch in patches:
        pred = model.predict(patch.features()).label
        confusion[patch.label][pred] += 1
        if pred == patch.label:
            correct += 1
            tp[pred] += 1
        else:
            fp[pred] += 1
            fn[patch.label] += 1
        if pred == "weed":
            targeted += 1

    def safe_div(a: int, b: int) -> float:
        return a / b if b else 0.0

    precision = {c: safe_div(tp[c], tp[c] + fp[c]) for c in LABELS}
    recall = {c: safe_div(tp[c], tp[c] + fn[c]) for c in LABELS}
    f1 = {
        c: safe_div(2 * precision[c] * recall[c] * 1, (precision[c] + recall[c]) or 1)
        for c in LABELS
    }
    blanket = len(patches)
    reduction = 100.0 * (blanket - targeted) / blanket if blanket else 0.0

    return Metrics(
        precision=precision,
        recall=recall,
        f1=f1,
        confusion={t: {p: str(confusion[t][p]) for p in LABELS} for t in LABELS},
        accuracy=safe_div(correct, len(patches)),
        pesticide_reduction_pct=reduction,
        blanket_treatments=blanket,
        targeted_treatments=targeted,
    )
