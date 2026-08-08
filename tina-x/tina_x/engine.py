"""TINA-X reasoning engine: load the graph + rules, inject events, query cascades.

For a junior dev
----------------
This is the Python front door to the MeTTa reasoner. It:
  1. loads the knowledge graph (metta/graph.metta) and cascade rules (metta/cascade.metta),
  2. lets you INJECT an event by changing a node's `status` (MeTTa has no in-place update, so we
     remove the old status atom and add the new one),
  3. QUERIES the derived predicates (hospital-critical, hospital-isolated, datacenter-down) to
     produce a list of predicted failures.

It deliberately mirrors Nzi's `SubprocessBrain` idea (drive MeTTa from another language) but is a
LOCAL copy — TINA-X does not import any Nzi code (see ADR 0005).
"""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

# metta/ lives one level up from this package (tina-x/metta/).
_METTA_DIR = Path(__file__).resolve().parent.parent / "metta"


@dataclass(frozen=True)
class Failure:
    """One predicted failure, with enough context to explain it to an emergency manager."""

    kind: str  # e.g. "hospital-critical", "hospital-isolated", "datacenter-down"
    node: str  # the affected node id, e.g. "hospital-B"

    def message(self) -> str:
        pretty = {
            "hospital-critical": "has LOST POWER (grid down and backup generator cannot run)",
            "hospital-isolated": "is ISOLATED (has power but no usable access road)",
            "datacenter-down": "is DOWN (grid failure, no backup) — cloud services at risk",
        }.get(self.kind, self.kind)
        return f"{self.node} {pretty}"


class TinaEngine:
    """Wraps a MeTTa instance loaded with the TINA-X graph + cascade rules."""

    # The nodes we scan for each derived failure predicate. Kept explicit (rather than scraped
    # from the graph) so the demo/tests are deterministic and easy to read.
    HOSPITALS = ("hospital-B", "hospital-C")
    DATACENTERS = ("datacenter-1",)

    def __init__(self, metta_dir: Path | None = None):
        from hyperon import MeTTa  # imported here so a missing hyperon fails loudly at use-time

        self._metta_dir = metta_dir or _METTA_DIR
        self.metta = MeTTa()
        # Load graph first (facts), then rules (logic).
        self.metta.run((self._metta_dir / "graph.metta").read_text())
        self.metta.run((self._metta_dir / "cascade.metta").read_text())

    # --- event injection ---------------------------------------------------------------------

    def set_status(self, node: str, new_state: str) -> None:
        """Change a node's live status (remove the old `status` atom, add the new one).

        MeTTa has no in-place mutation, so we delete then insert. If there was no prior status,
        the remove is a harmless no-op.
        """
        # Find the current status (if any) so we can remove that exact atom.
        current = self._one(f"!(status-of {node})")
        if current is not None:
            self.metta.run(f"!(remove-atom &self (status {node} {current}))")
        self.metta.run(f"(status {node} {new_state})")

    def inject(self, changes: dict[str, str]) -> None:
        """Apply a set of {node: new_status} changes — i.e. a 'black swan' event's effects."""
        for node, state in changes.items():
            self.set_status(node, state)

    # --- queries -----------------------------------------------------------------------------

    def status_of(self, node: str) -> str | None:
        """Current live status of a node, or None if unknown."""
        return self._one(f"!(status-of {node})")

    def cascade(self) -> list[Failure]:
        """Run all cascade predicates over the known nodes; return the predicted failures."""
        failures: list[Failure] = []

        for h in self.HOSPITALS:
            if self._nonempty(f"!(hospital-critical {h})"):
                failures.append(Failure("hospital-critical", h))
            elif self._nonempty(f"!(hospital-isolated {h})"):
                failures.append(Failure("hospital-isolated", h))

        for d in self.DATACENTERS:
            if self._nonempty(f"!(datacenter-down {d})"):
                failures.append(Failure("datacenter-down", d))

        return failures

    # --- low-level MeTTa helpers -------------------------------------------------------------

    def _one(self, query: str) -> str | None:
        """Run a query expected to yield a single atom; return it as a string, or None."""
        results = self.metta.run(query)
        for result_list in results:
            for atom in result_list:
                return str(atom)
        return None

    def _nonempty(self, query: str) -> bool:
        """True if the query returned a real value (our rules return `(empty)` for 'no failure').

        A derived predicate like `hospital-critical` returns the node id when the failure holds,
        and the atom `()`/`empty` otherwise. We treat those empties as 'no failure'.
        """
        val = self._one(query)
        if val is None:
            return False
        stripped = val.strip()
        return stripped not in ("(empty)", "empty", "()", "")
