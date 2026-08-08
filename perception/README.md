# `perception/` — precision-ag weed/pest detection + verifiable treat decision (Phase 3)

The economically-real task, built to run **here** (stdlib-only, deterministic, no downloads): a
bot perceives crop imagery and makes a **symbolically justified** treat/don't-treat decision. The
ML side perceives; MeTTa decides and explains — the same verifiability principle as the Phase-2
moat, now applied to spraying pesticide (an irreversible, costly action).

## Pipeline

```
synth.py    procedurally generate labeled field patches (crop/weed/soil)   [P3.1 data]
   |          interpretable channels: green_frac, row_align, leaf_broad, ndvi_var
   v
model.py    stdlib multiclass logistic regression -> Detection(class, confidence)   [P3.1]
   |          + evaluate(): precision/recall/F1, confusion, pesticide-reduction %    [P3.4]
   v
decide.py   Detection -> MeTTa agronomy rules -> Verdict(treat|skip, reason)   [P3.2 fusion]
              (metta-logic/agronomy/treat.metta: spray ONLY a confident weed;
               fail-closed — an uncertain weed is skipped, never sprayed on a guess)
```

## Run

```bash
python3 perception/pipeline_demo.py        # train + honest metrics + symbolic decisions
python3 perception/tests/test_perception.py # tests (symbolic ones skip w/o hyperon)
```

## Measured (synthetic held-out set, seeds train=1 / test=2)

- accuracy ~0.88, weed F1 ~0.85 (task tuned to OVERLAP so the score is honest, not ~1.0),
- **~70% pesticide reduction** vs. a blanket spray — the economic story, and the symbolic
  confidence gate can only ever spray *less*, never more.

## What's deferred

- **P3.3 → Phase 7**: the Unity synthetic-data pipeline for long-tail perception cases (needs the
  Unity engine). This `synth.py` is the honest headless stand-in that lets us build + measure the
  decision pipeline now; the real detector + 3D data generation land with Unity.
