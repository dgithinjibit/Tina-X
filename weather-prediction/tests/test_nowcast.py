"""Project Nzi — weather nowcasting tests (P5.1/P5.2/P5.3/P5.5), stdlib only.

Verifies the synthetic data, the honest metrics, and that swarm fusion + symbolic calibration
actually help. Symbolic (MeTTa) tests skip gracefully without hyperon.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from nowcast import ensemble, ensemble_prob, swarm_prob, truth_at  # noqa: E402
from thermodynamic import energy, thermodynamic_prob  # noqa: E402
from verify import brier_score, ets, reliability  # noqa: E402

SIZE = 16
MEMBERS = 20
SEED = 7


# --- data (P5.1/P5.2) -----------------------------------------------------------------------

def test_truth_is_deterministic_and_has_a_storm():
    a = truth_at(3, SIZE)
    b = truth_at(3, SIZE)
    assert a == b, "ground truth must be deterministic"
    assert 0 < a.positives() < SIZE * SIZE, "a storm exists but doesn't fill the grid"


def test_ensemble_prob_is_a_probability_field():
    members = ensemble(3, SIZE, MEMBERS, SEED)
    prob = ensemble_prob(members)
    flat = [p for row in prob for p in row]
    assert all(0.0 <= p <= 1.0 for p in flat)
    assert any(p > 0.0 for p in flat), "some cells should carry hazard probability"


# --- metrics are honest (P5.5) --------------------------------------------------------------

def test_brier_rewards_a_good_forecast_over_a_useless_one():
    truth = truth_at(3, SIZE)
    good = ensemble_prob(ensemble(3, SIZE, MEMBERS, SEED))
    # A useless "always 0.5" forecast scores Brier 0.25; the real forecast must beat it.
    useless = [[0.5] * SIZE for _ in range(SIZE)]
    assert brier_score(good, truth) < brier_score(useless, truth)


def test_ets_is_not_fooled_by_the_all_zero_forecast():
    # The rare-event trap: "never hazard" is ~99% accurate but has ZERO skill. ETS must catch this.
    truth = truth_at(3, SIZE)
    all_zero = [[0.0] * SIZE for _ in range(SIZE)]
    assert abs(ets(all_zero, truth)) < 1e-9, "no-skill forecast must score ETS ~0, not high"
    good = ensemble_prob(ensemble(3, SIZE, MEMBERS, SEED))
    assert ets(good, truth) > 0.2, "a real forecast should show positive skill"


def test_swarm_fusion_improves_brier_where_sensors_are():
    truth = truth_at(3, SIZE)
    base = ensemble_prob(ensemble(3, SIZE, MEMBERS, SEED))
    positions = [(i, i) for i in range(SIZE)]  # a diagonal of in-situ sensors
    fused = swarm_prob(base, truth, positions)
    # In-situ observations along the swarm path should reduce Brier (better local truth).
    assert brier_score(fused, truth) <= brier_score(base, truth)


# --- symbolic calibration (P5.3) ------------------------------------------------------------

def _skip_if_no_hyperon():
    try:
        import hyperon  # noqa: F401
        return False
    except ModuleNotFoundError:
        print("SKIP symbolic tests: hyperon not installed (activate the .venv).")
        return True


def test_symbolic_calibration_improves_reliability():
    if _skip_if_no_hyperon():
        return
    from hyperon import MeTTa
    from symbolic import apply_guidance

    truth = truth_at(3, SIZE)
    prev = truth_at(2, SIZE)
    members = ensemble(3, SIZE, MEMBERS, SEED)
    base = ensemble_prob(members)
    calibrated = apply_guidance(base, members, prev, MeTTa())

    base_rel, _ = reliability(base, truth)
    cal_rel, _ = reliability(calibrated, truth)
    # Shrinking overconfident extremes toward climatology should not worsen calibration.
    assert cal_rel <= base_rel + 1e-6, f"reliability worsened: {base_rel} -> {cal_rel}"


# --- thermodynamic / pbit sampling (P5.4) ---------------------------------------------------

def test_thermodynamic_sampler_is_a_probability_field_and_deterministic():
    base = ensemble_prob(ensemble(3, SIZE, MEMBERS, SEED))
    a = thermodynamic_prob(base, seed=SEED)
    b = thermodynamic_prob(base, seed=SEED)
    assert a == b, "same seed must give the same posterior (deterministic sampling)"
    flat = [p for row in a for p in row]
    assert all(0.0 <= p <= 1.0 for p in flat), "posterior must be a probability field"


def test_thermodynamic_denoising_improves_brier_and_reliability():
    # The honest win: neighbor-coupled pbit sampling denoises per-cell ensemble speckle. With the
    # gentle default coupling, Brier AND the reliability penalty improve at every lead time.
    for t in (1, 5, 10):
        truth = truth_at(t, SIZE)
        base = ensemble_prob(ensemble(t, SIZE, MEMBERS, SEED))
        th = thermodynamic_prob(base, seed=SEED)
        assert brier_score(th, truth) <= brier_score(base, truth) + 1e-9, f"Brier worse at t={t}"
        assert reliability(th, truth)[0] <= reliability(base, truth)[0] + 1e-9, f"reliability worse at t={t}"


def test_zero_coupling_recovers_the_per_cell_forecast():
    # beta_smooth=0 removes the spatial prior, so the sampler must not manufacture spatial structure:
    # its posterior should track the input probability closely (no systematic denoising).
    base = ensemble_prob(ensemble(3, SIZE, MEMBERS, SEED))
    th = thermodynamic_prob(base, beta_smooth=0.0, sweeps=120, burn_in=30, seed=SEED)
    truth = truth_at(3, SIZE)
    # With no coupling it should be no better than the base at ETS (it isn't denoising anything).
    assert abs(ets(th, truth) - ets(base, truth)) < 0.15, "zero-coupling should not change skill much"


def test_energy_prefers_the_coherent_field_over_a_speckled_one():
    # A connected blob (low boundary) must have lower energy than the same mass scattered as speckle.
    base = ensemble_prob(ensemble(3, SIZE, MEMBERS, SEED))
    blob = [[1 if 4 <= x <= 7 and 4 <= y <= 7 else 0 for x in range(SIZE)] for y in range(SIZE)]
    speckle = [[1 if (x * 3 + y * 5) % 6 == 0 else 0 for x in range(SIZE)] for y in range(SIZE)]
    e_blob = energy(base, blob, beta_data=1.2, beta_smooth=0.2)
    e_speck = energy(base, speckle, beta_data=1.2, beta_smooth=0.2)
    assert e_blob < e_speck, "the smoothness prior must favor a coherent storm over scattered speckle"


if __name__ == "__main__":
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
