"""TINA-X demo — run black-swan scenarios and print the predicted cascades.

Run:  .venv/bin/python -m tina_x.demo
      .venv/bin/python -m tina_x.demo --scenario quake-typhoon
      .venv/bin/python -m tina_x.demo --tina-url http://127.0.0.1:8080   # also push to TINA-X (optional)

The headline result to look for: individually survivable events produce NO failures, but the
COMPOUND event cascades into a hospital losing power — deduced, never trained on.
"""
from __future__ import annotations

import argparse
import sys

from .engine import TinaEngine
from .scenarios import ALL


def run_scenario(key: str, tina_url: str | None = None) -> int:
    """Run one scenario end-to-end; returns the number of predicted failures."""
    scenario = ALL[key]
    print(f"\n=== {scenario.name} ===")
    print(f"    {scenario.description}")

    # Each scenario starts from a FRESH graph so runs don't contaminate each other.
    engine = TinaEngine()
    engine.inject(scenario.changes)

    changed = ", ".join(f"{k}->{v}" for k, v in scenario.changes.items())
    print(f"    injected: {changed}")

    failures = engine.cascade()
    if not failures:
        print("    ✅ no cascading failures predicted")
    else:
        print(f"    🚨 {len(failures)} cascading failure(s) predicted:")
        for f in failures:
            print(f"       - {f.message()}")

    # OPTIONAL: forward alerts to the TINA-X dashboard. Never required (ADR 0005).
    if tina_url and failures:
        from .bridge import push_alerts

        ok = push_alerts(failures, tina_url, scenario.name)
        print(f"    bridge: {'sent to TINA-X' if ok else 'TINA-X unreachable (ignored)'}")

    return len(failures)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="TINA-X cascading-failure demo")
    parser.add_argument(
        "--scenario",
        choices=sorted(ALL.keys()),
        help="run a single scenario (default: run them all)",
    )
    parser.add_argument(
        "--tina-url",
        default=None,
        help="optional TINA-X dashboard URL to push alerts to (e.g. http://127.0.0.1:8080)",
    )
    args = parser.parse_args(argv)

    print("TINA-X — a Digital Twin of Society's Fragility")
    print("Predicting the collapse of the systems humans rely on, not just the hazard.")

    keys = [args.scenario] if args.scenario else sorted(ALL.keys())
    for key in keys:
        run_scenario(key, args.tina_url)

    print(
        "\nNote: the single-hazard scenarios are survivable; the COMPOUND ones cascade. "
        "TINA-X deduced that from the dependency graph — it was never trained on it."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
