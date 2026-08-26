# 🌍 TINA-X — a Digital Twin of Society's Fragility

> **TINA-X is an independent component of Project TINA-X** (see `../docs/adr/0005-...`). You can work
> on and run it *without* the rest of TINA-X. It can optionally push alerts to the TINA-X dashboard, but
> never requires it.

## What it is
Most disaster tools predict the *hazard* (the wind, the quake). TINA-X predicts the **collapse of
the systems humans rely on to survive the hazard** — the cascading domino effect across
infrastructure.

It ingests **infrastructure dependencies** into a MeTTa Atomspace and uses forward-chaining
**symbolic rules** to deduce catastrophic cascades from compound "black swan" events — including
combinations it has never seen (where deep learning fails).

**Example it reasons through:**
> Earthquake → power grid offline → hospital switches to backup generator → generator needs diesel
> → diesel needs road delivery → typhoon floods the road → **hospital fails.**
> A naive model sees "hospital has a generator, it's fine." TINA-X follows the chain and flags it.

## Why MeTTa (symbolic), not deep learning
Deep learning does pattern recognition ("this radar looks like a tornado") but fails on
**out-of-distribution** compound events. TINA-X uses a **knowledge graph + logic**, so it can
*deduce* a novel cascade from known dependencies. Every alert is **explainable** — traceable to the
rule and facts that produced it.

## Run it standalone
```bash
cd tina-x
python3 -m venv .venv
.venv/bin/pip install hyperon pytest
.venv/bin/python -m tina_x.demo          # run a black-swan scenario, print the cascade
.venv/bin/python -m pytest               # tests
```

## Layout
```
tina-x/
├── metta/                 # the knowledge graph + cascading-failure rules (MeTTa)
│   ├── graph.metta        # infrastructure atoms + dependency edges (a small Kyushu-like region)
│   └── cascade.metta      # forward-chaining failure-propagation rules
├── tina_x/                # Python package: inject events, run cascades, format alerts
│   ├── engine.py          # loads MeTTa, injects events, queries failures
│   ├── scenarios.py       # named "black swan" events (quake, typhoon, solar flare, combos)
│   ├── demo.py            # runnable end-to-end demo
│   └── bridge.py          # OPTIONAL: push alerts to the TINA-X dashboard (no-op if TINA-X absent)
└── tests/                 # pytest
```

## Boundary (the rule that keeps it independent)
TINA-X may **send** alerts to TINA-X over HTTP/JSON; it must never **require** TINA-X. See ADR 0005.
