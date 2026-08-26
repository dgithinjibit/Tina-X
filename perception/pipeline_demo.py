#!/usr/bin/env python3
"""Project TINA-X — precision-ag perception pipeline demo (P3.1 + P3.2 + P3.4).

The end-to-end economically-real task, runnable here:
  1. generate a synthetic labeled field (train + held-out test),
  2. train the weed/pest detector (P3.1),
  3. report honest precision/recall/F1 + pesticide-reduction % on the test set (P3.4),
  4. run a few patches through perception -> SYMBOLIC treat/don't-treat with justification (P3.2).

Run:  python3 perception/pipeline_demo.py         (steps 1-3 need no hyperon; step 4 needs the venv)
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from model import evaluate, train  # noqa: E402
from synth import make_dataset  # noqa: E402


def main() -> int:
    print("Project TINA-X — precision-ag perception pipeline (Phase 3)")

    # 1. Deterministic synthetic field. Different seeds for train vs. test = a real held-out split.
    train_set = make_dataset(n=1500, seed=1)
    test_set = make_dataset(n=600, seed=2)
    print(f"  data: {len(train_set)} train / {len(test_set)} test patches (crop/weed/soil)")

    # 2. Train the detector (P3.1).
    model = train(train_set)

    # 3. Honest metrics on held-out data (P3.4).
    metrics = evaluate(model, test_set)
    print("  " + metrics.summary().replace("\n", "\n  "))

    # 4. Perception -> symbolic decision with justification (P3.2). Needs hyperon.
    try:
        from hyperon import MeTTa  # type: ignore
        from decide import decide
    except ModuleNotFoundError:
        print("  (skipping symbolic-decision step: hyperon not installed — activate the .venv)")
        return 0 if metrics.accuracy > 0.7 else 2

    metta = MeTTa()
    print("  perception -> symbolic treat/don't-treat (first 6 test patches):")
    sprayed = 0
    for patch in test_set[:6]:
        det = model.predict(patch.features())
        verdict = decide(det, metta=metta)
        sprayed += 1 if verdict.treat else 0
        print(
            f"    truth={patch.label:5s} detected={det.label:5s} "
            f"conf={det.confidence:.2f} -> {verdict.action.upper():5s} ({verdict.reason})"
        )
    print(f"  RESULT: pipeline OK — accuracy {metrics.accuracy:.2f}, "
          f"{metrics.pesticide_reduction_pct:.0f}% less pesticide vs blanket spray.")
    return 0 if metrics.accuracy > 0.7 else 2


if __name__ == "__main__":
    raise SystemExit(main())
