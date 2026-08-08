"""Named 'black swan' scenarios — the compound events TINA-X reasons about.

For a junior dev: each scenario is just a set of status changes to inject into the graph. The
POWER of TINA-X is COMBINING them: individually survivable events that together cascade. Add your
own scenarios here; the engine + rules handle the reasoning.
"""
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Scenario:
    """A named event and the status changes it causes on the graph."""

    name: str
    description: str
    changes: dict[str, str]


# --- single-hazard events (each individually survivable) -------------------------------------

EARTHQUAKE = Scenario(
    name="Kumamoto Earthquake",
    description="Seismic event knocks power grid A offline.",
    changes={"grid-A": "offline"},
)

TYPHOON = Scenario(
    name="Typhoon Dolphin",
    description="Flooding makes road 3 impassable.",
    changes={"road-3": "flooded"},
)

SOLAR_FLARE = Scenario(
    name="Carrington-class CME",
    description="Coronal mass ejection trips substation X, taking grid A offline.",
    changes={"substation-X": "offline", "grid-A": "offline"},
)


# --- COMPOUND events (the ones that actually cause cascades) ---------------------------------

QUAKE_THEN_TYPHOON = Scenario(
    name="Earthquake + Typhoon (compound)",
    description=(
        "The exact 'never trained on this' case: grid A fails from the quake, so the hospital "
        "switches to its backup generator — but the typhoon has flooded the only fuel-delivery "
        "road, so the generator cannot be refuelled. The hospital fails."
    ),
    changes={"grid-A": "offline", "road-3": "flooded"},
)

CYBER_PHYSICAL_CASCADE = Scenario(
    name="Solar flare → grid → data center → cloud",
    description=(
        "A CME trips substation X and grid A. The data center it powers has no backup and goes "
        "down, threatening the cloud services (and hospital logistics software) that depend on it."
    ),
    changes={"substation-X": "offline", "grid-A": "offline"},
)


# Registry so the demo / CLI can look scenarios up by key.
ALL: dict[str, Scenario] = {
    "earthquake": EARTHQUAKE,
    "typhoon": TYPHOON,
    "solar-flare": SOLAR_FLARE,
    "quake-typhoon": QUAKE_THEN_TYPHOON,
    "cyber-cascade": CYBER_PHYSICAL_CASCADE,
}
