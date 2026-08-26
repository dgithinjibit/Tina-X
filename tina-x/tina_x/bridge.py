"""OPTIONAL bridge: push TINA-X alerts to the TINA-X dashboard.

Boundary rule (ADR 0005): TINA-X may SEND to TINA-X, but must NEVER REQUIRE it. So this module:
  * uses only the Python standard library (no TINA-X imports, no third-party deps),
  * returns False (never raises) if TINA-X is unreachable,
  * is only ever called when the user explicitly passes --tina-url.

The contract is a simple HTTP POST of JSON alerts. If/when the TINA-X server adds a matching
`/api/alerts` endpoint, this lights up; until then it degrades gracefully.
"""
from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Iterable

from .engine import Failure


def push_alerts(failures: Iterable[Failure], tina_url: str, source: str, timeout: float = 2.0) -> bool:
    """POST alerts to `<tina_url>/api/alerts`. Return True on success, False on any failure.

    We swallow ALL network errors on purpose: TINA-X must keep working whether or not TINA-X is up.
    """
    payload = {
        "source": source,
        "alerts": [{"kind": f.kind, "node": f.node, "message": f.message()} for f in failures],
    }
    # Wrap EVERYTHING — even Request construction, which raises ValueError on a malformed URL —
    # so a bad/unreachable TINA-X never breaks TINA-X.
    try:
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url=f"{tina_url.rstrip('/')}/api/alerts",
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return 200 <= resp.status < 300
    except (urllib.error.URLError, OSError, ValueError):
        # TINA-X not running / endpoint missing / bad URL — all fine, TINA-X carries on.
        return False
