#!/usr/bin/env python3
"""Project Nzi — MeTTa bridge worker (called by the Rust SubprocessBrain).

WHAT THIS IS (for a junior dev):
  The Rust "slow brain" needs to ask MeTTa questions. Rust cannot (yet) link the MeTTa
  library directly (see docs/adr/0003-rust-metta-bridge.md), so instead Rust runs THIS
  script as a subprocess, hands it a MeTTa query, and reads back the answer.

THE CONTRACT (this is a promise Rust depends on — do not break it lightly):
  * INPUT  : the MeTTa query text is passed as the first command-line argument.
             (Using argv keeps it simple; a future long-lived worker could read stdin.)
  * OUTPUT : exactly one line of JSON is printed to STDOUT, of the form:
                 {"ok": true,  "results": ["True"]}         # success
                 {"ok": false, "error": "message here"}     # failure
  * EXIT   : 0 on success, non-zero on failure. Rust checks both the exit code AND "ok".

  Keeping the output to a single JSON line makes it trivial and unambiguous for Rust to parse.

EXAMPLE:
  .venv/bin/python metta-logic/bridge_worker.py '!(+ 1 2)'
  -> {"ok": true, "results": ["3"]}
"""
from __future__ import annotations

import json
import sys


def run_query(query_text: str) -> dict:
    """Run one MeTTa query and return a plain dict following the OUTPUT contract above.

    We import hyperon *inside* the function so that if it is missing we can still return a
    clean JSON error instead of crashing with a traceback (Rust expects JSON, always).
    """
    try:
        from hyperon import MeTTa  # type: ignore
    except ModuleNotFoundError:
        return {"ok": False, "error": "hyperon not installed (activate the .venv)"}

    try:
        metta = MeTTa()
        # metta.run returns a list of result-lists (one per '!' query line). We flatten every
        # atom to its string form so the boundary carries only simple, language-neutral text.
        raw_results = metta.run(query_text)
        results: list[str] = []
        for result_list in raw_results:
            for atom in result_list:
                results.append(str(atom))
        return {"ok": True, "results": results}
    except Exception as exc:  # noqa: BLE001 — we deliberately convert ANY error into JSON
        return {"ok": False, "error": f"{type(exc).__name__}: {exc}"}


def main(argv: list[str]) -> int:
    # argv[0] is the script name; argv[1] should be the query. Guard against missing input.
    if len(argv) < 2:
        print(json.dumps({"ok": False, "error": "no query argument provided"}))
        return 2

    query_text = argv[1]
    payload = run_query(query_text)

    # Emit EXACTLY one JSON line. This is what Rust parses.
    print(json.dumps(payload))

    # Exit code mirrors the "ok" flag so Rust can fail fast on process status too.
    return 0 if payload.get("ok") else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
