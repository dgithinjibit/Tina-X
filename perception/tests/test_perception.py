"""Project Nzi — precision-ag perception tests (P3.1/P3.2/P3.4).

Pure-stdlib tests for the synthetic generator, the detector's measured quality, and the symbolic
treat/don't-treat logic. The MeTTa-backed tests SKIP gracefully when hyperon isn't installed, like
the Rust integration tests — so the suite is green everywhere but proves the real path where the
venv exists.

Run:  python3 -m pytest perception/tests/        (or) python3 perception/tests/test_perception.py
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from model import Detection, evaluate, train  # noqa: E402
from synth import LABELS, make_dataset  # noqa: E402


# --- synth (P3.1) ---------------------------------------------------------------------------

def test_dataset_is_deterministic_in_seed():
    a = make_dataset(200, seed=7)
    b = make_dataset(200, seed=7)
    assert [p.features() for p in a] == [p.features() for p in b]
    # A different seed yields a different field.
    c = make_dataset(200, seed=8)
    assert [p.features() for p in a] != [p.features() for p in c]


def test_dataset_size_and_labels():
    ds = make_dataset(100, seed=1)
    assert len(ds) == 100
    assert set(p.label for p in ds) <= set(LABELS)
    assert all(0.0 <= v <= 1.0 for p in ds for v in p.features())


# --- model + metrics (P3.1/P3.4) ------------------------------------------------------------

def test_model_beats_baseline_and_is_honest():
    model = train(make_dataset(1500, seed=1))
    m = evaluate(model, make_dataset(600, seed=2))
    # Must clearly beat majority-class guessing (~0.5 crop) but NOT be a suspicious ~1.0 (which
    # would mean the synthetic task is too easy to be meaningful).
    assert 0.75 <= m.accuracy < 0.99, f"accuracy {m.accuracy} outside the honest band"
    # Weed detection is the economically important class — demand usable precision AND recall.
    assert m.precision["weed"] > 0.7
    assert m.recall["weed"] > 0.7


def test_pesticide_reduction_is_real_and_bounded():
    model = train(make_dataset(1500, seed=1))
    m = evaluate(model, make_dataset(600, seed=2))
    # Targeted spraying must save a meaningful, sane fraction vs. blanket (weeds are a minority).
    assert 30.0 < m.pesticide_reduction_pct < 90.0
    assert m.targeted_treatments < m.blanket_treatments


# --- symbolic decision (P3.2) ---------------------------------------------------------------

def _skip_if_no_hyperon():
    try:
        import hyperon  # noqa: F401
        return False
    except ModuleNotFoundError:
        print("SKIP symbolic-decision tests: hyperon not installed (activate the .venv).")
        return True


def test_symbolic_confident_weed_is_treated():
    if _skip_if_no_hyperon():
        return
    from decide import decide
    v = decide(Detection(label="weed", probs={"weed": 0.82, "crop": 0.1, "soil": 0.08}, confidence=0.82))
    assert v.treat and v.reason == "confident-weed"


def test_symbolic_low_confidence_weed_is_skipped():
    if _skip_if_no_hyperon():
        return
    from decide import decide
    # Below the 0.60 threshold: refuse to spray on a guess (fail-closed, saves pesticide).
    v = decide(Detection(label="weed", probs={"weed": 0.45, "crop": 0.3, "soil": 0.25}, confidence=0.45))
    assert not v.treat and v.reason == "low-confidence-weed"


def test_symbolic_crop_and_soil_are_skipped():
    if _skip_if_no_hyperon():
        return
    from decide import decide
    for label in ("crop", "soil"):
        v = decide(Detection(label=label, probs={label: 0.9, "weed": 0.05, "crop": 0.05}, confidence=0.9))
        assert not v.treat and v.reason == "no-weed-detected"


if __name__ == "__main__":
    # Minimal runner so this works without pytest installed.
    fns = [g for name, g in sorted(globals().items()) if name.startswith("test_") and callable(g)]
    failures = 0
    for fn in fns:
        try:
            fn()
            print(f"  [OK ] {fn.__name__}")
        except AssertionError as e:
            failures += 1
            print(f"  [BAD] {fn.__name__}: {e}")
    print(f"  RESULT: {'ALL PASS' if failures == 0 else f'{failures} FAILED'}")
    raise SystemExit(0 if failures == 0 else 1)
