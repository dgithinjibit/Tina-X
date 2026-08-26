"""Tests for the OPTIONAL TINA-X bridge — it must degrade gracefully, never raise (ADR 0005)."""
from __future__ import annotations

from tina_x.bridge import push_alerts
from tina_x.engine import Failure


def test_push_to_unreachable_tina_returns_false_not_raises():
    # Point at a port nothing is listening on. Must return False, must NOT raise — TINA-X keeps
    # working whether or not TINA-X is up.
    failures = [Failure("hospital-critical", "hospital-B")]
    result = push_alerts(failures, "http://127.0.0.1:59999", source="unit-test", timeout=0.5)
    assert result is False


def test_push_with_bad_url_returns_false():
    failures = [Failure("datacenter-down", "datacenter-1")]
    assert push_alerts(failures, "not-a-real-url", source="unit-test", timeout=0.5) is False
