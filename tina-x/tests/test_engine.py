"""Tests for the TINA-X reasoning engine.

These run WITHOUT any part of TINA-X present (ADR 0005): pure hyperon + local code.
"""
from __future__ import annotations

from tina_x.engine import Failure, TinaEngine
from tina_x import scenarios


def fresh() -> TinaEngine:
    return TinaEngine()


def kinds(failures: list[Failure]) -> set[str]:
    return {f"{f.node}:{f.kind}" for f in failures}


def test_nominal_state_has_no_failures():
    # With everything nominal, TINA-X must predict no cascades (no false alarms).
    assert fresh().cascade() == []


def test_single_earthquake_is_survivable():
    # Grid A offline ALONE: hospital-B falls back to its generator, whose fuel road is still open.
    e = fresh()
    e.inject(scenarios.EARTHQUAKE.changes)
    failures = kinds(e.cascade())
    # hospital-B should NOT be power-critical (backup works)...
    assert "hospital-B:hospital-critical" not in failures
    # ...though the data center (no backup) does go down on grid loss.
    assert "datacenter-1:datacenter-down" in failures


def test_single_typhoon_is_survivable_for_the_hospital():
    # Road 3 flooded ALONE: grid is still up, so the hospital keeps mains power.
    e = fresh()
    e.inject(scenarios.TYPHOON.changes)
    assert "hospital-B:hospital-critical" not in kinds(e.cascade())


def test_compound_quake_plus_typhoon_cascades_to_hospital_failure():
    # THE headline case: neither event alone kills the hospital, but TOGETHER they do
    # (grid down -> backup -> needs fuel -> road flooded -> generator dead -> hospital critical).
    e = fresh()
    e.inject(scenarios.QUAKE_THEN_TYPHOON.changes)
    assert "hospital-B:hospital-critical" in kinds(e.cascade())


def test_unaffected_hospital_stays_healthy():
    # hospital-C (grid-B / road-5) must be untouched by an event on grid-A / road-3.
    e = fresh()
    e.inject(scenarios.QUAKE_THEN_TYPHOON.changes)
    ks = kinds(e.cascade())
    assert "hospital-C:hospital-critical" not in ks
    assert "hospital-C:hospital-isolated" not in ks


def test_cyber_physical_cascade_takes_down_datacenter():
    # Solar flare -> substation + grid A down -> data center (no backup) down.
    e = fresh()
    e.inject(scenarios.CYBER_PHYSICAL_CASCADE.changes)
    assert "datacenter-1:datacenter-down" in kinds(e.cascade())


def test_set_status_updates_are_readable():
    e = fresh()
    assert e.status_of("grid-A") == "online"
    e.set_status("grid-A", "offline")
    assert e.status_of("grid-A") == "offline"


def test_failure_message_is_human_readable():
    f = Failure("hospital-critical", "hospital-B")
    assert "LOST POWER" in f.message()
    assert "hospital-B" in f.message()


def test_isolation_when_powered_but_road_cut():
    # If the hospital keeps power (grid up) but its access road is destroyed, it's ISOLATED,
    # not power-critical. Grid stays online; only the access road is cut.
    e = fresh()
    e.set_status("road-3", "destroyed")
    ks = kinds(e.cascade())
    assert "hospital-B:hospital-isolated" in ks
    assert "hospital-B:hospital-critical" not in ks
