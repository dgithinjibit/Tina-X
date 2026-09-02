"""TINA-X FastAPI server — exposes the cascading-failure engine over HTTP/JSON.

Endpoints
---------
GET  /api/health            liveness probe
GET  /api/scenarios         list all named scenarios with metadata
GET  /api/graph             full infrastructure graph (nodes + edges)
POST /api/simulate          run a named scenario OR a custom event set; return cascade result
POST /api/alerts            receive alerts from TINA-X bridge (passthrough / store latest)

Run (dev):
    .venv/bin/uvicorn tina_x.api:app --reload --port 8080

For Vercel / production the frontend is static; this API is only needed when a live backend
is reachable. The frontend degrades gracefully to mock data when the API is absent.
"""
from __future__ import annotations

from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from .engine import TinaEngine
from .scenarios import ALL, Scenario

app = FastAPI(
    title="TINA-X API",
    description="Digital Twin of Society's Fragility — cascading-failure symbolic reasoner",
    version="0.1.0",
)

# Allow the frontend (any origin in dev; tighten in prod) to call the API.
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# ---- latest alerts store (in-memory, reset on restart) ---------------------------------------
_latest_alerts: list[dict] = []


# ---- Pydantic models -------------------------------------------------------------------------

class SimulateRequest(BaseModel):
    """Either supply a known scenario key OR a custom `changes` dict."""
    scenario: str | None = None          # e.g. "quake-typhoon"
    changes: dict[str, str] | None = None  # e.g. {"grid-A": "offline", "road-3": "flooded"}


class AlertsPayload(BaseModel):
    source: str
    alerts: list[dict[str, Any]]


# ---- helpers ---------------------------------------------------------------------------------

def _scenario_meta(key: str, s: Scenario) -> dict:
    return {
        "key": key,
        "name": s.name,
        "description": s.description,
        "changes": s.changes,
    }


def _run_engine(changes: dict[str, str]) -> dict:
    """Spin up a fresh engine, inject changes, return cascade result."""
    engine = TinaEngine()
    engine.inject(changes)
    failures = engine.cascade()

    # Also snapshot the current status of every known node.
    all_nodes = list(engine.HOSPITALS) + list(engine.DATACENTERS) + [
        "grid-A", "grid-B", "substation-X", "road-3", "road-5",
    ]
    statuses = {n: engine.status_of(n) for n in all_nodes}

    return {
        "failures": [
            {"kind": f.kind, "node": f.node, "message": f.message()}
            for f in failures
        ],
        "statuses": statuses,
        "failure_count": len(failures),
    }


# ---- routes ----------------------------------------------------------------------------------

@app.get("/api/health")
def health() -> dict:
    return {"status": "ok", "service": "tina-x"}


@app.get("/api/scenarios")
def list_scenarios() -> dict:
    return {
        "scenarios": [_scenario_meta(k, v) for k, v in ALL.items()]
    }


@app.get("/api/graph")
def get_graph() -> dict:
    """Return the infrastructure graph as nodes + edges for the frontend visualiser."""
    nodes = [
        {"id": "grid-A",       "type": "PowerGrid",   "label": "Grid A"},
        {"id": "grid-B",       "type": "PowerGrid",   "label": "Grid B"},
        {"id": "substation-X", "type": "Substation",  "label": "Substation X"},
        {"id": "hospital-B",   "type": "Hospital",    "label": "Hospital B"},
        {"id": "hospital-C",   "type": "Hospital",    "label": "Hospital C"},
        {"id": "generator-1",  "type": "Generator",   "label": "Generator 1"},
        {"id": "generator-2",  "type": "Generator",   "label": "Generator 2"},
        {"id": "road-3",       "type": "Road",        "label": "Road 3"},
        {"id": "road-5",       "type": "Road",        "label": "Road 5"},
        {"id": "datacenter-1", "type": "DataCenter",  "label": "Data Center 1"},
    ]
    edges = [
        {"from": "substation-X", "to": "grid-A",       "rel": "feeds"},
        {"from": "grid-A",       "to": "hospital-B",   "rel": "powers"},
        {"from": "grid-A",       "to": "datacenter-1", "rel": "powers"},
        {"from": "grid-B",       "to": "hospital-C",   "rel": "powers"},
        {"from": "hospital-B",   "to": "generator-1",  "rel": "has-backup"},
        {"from": "hospital-C",   "to": "generator-2",  "rel": "has-backup"},
        {"from": "road-3",       "to": "generator-1",  "rel": "fuel-via"},
        {"from": "road-5",       "to": "generator-2",  "rel": "fuel-via"},
        {"from": "road-3",       "to": "hospital-B",   "rel": "road-access"},
        {"from": "road-5",       "to": "hospital-C",   "rel": "road-access"},
    ]
    return {"nodes": nodes, "edges": edges}


@app.post("/api/simulate")
def simulate(req: SimulateRequest) -> dict:
    if req.scenario and req.changes:
        raise HTTPException(400, "Supply either 'scenario' or 'changes', not both.")

    if req.scenario:
        if req.scenario not in ALL:
            raise HTTPException(404, f"Unknown scenario '{req.scenario}'. "
                                     f"Valid keys: {sorted(ALL.keys())}")
        changes = ALL[req.scenario].changes
        meta = _scenario_meta(req.scenario, ALL[req.scenario])
    elif req.changes:
        changes = req.changes
        meta = {"key": "custom", "name": "Custom Event", "description": "User-defined changes",
                "changes": changes}
    else:
        raise HTTPException(400, "Supply 'scenario' key or 'changes' dict.")

    result = _run_engine(changes)
    result["scenario"] = meta
    return result


@app.post("/api/alerts")
def receive_alerts(payload: AlertsPayload) -> dict:
    """Receive alerts pushed by the TINA-X bridge (ADR 0005 contract)."""
    global _latest_alerts
    _latest_alerts = payload.alerts
    return {"received": len(payload.alerts)}


@app.get("/api/alerts")
def get_latest_alerts() -> dict:
    return {"alerts": _latest_alerts}
