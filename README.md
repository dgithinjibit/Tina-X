# Project TINA-X — MeTTa reasoning for resilient autonomous systems

> **Nzi** means “fly” in Swahili. The fly is our control-system inspiration, not the product boundary. Project TINA-X is a MeTTa/Hyperon-centered research project that gives autonomous agents a symbolic reasoning and verification layer, then connects that layer to fast control, decentralized swarms, agriculture, weather, and infrastructure-resilience applications.

## The one-sentence pitch

**TINA-X turns MeTTa rules into an auditable supervisory brain for autonomous systems: the symbolic layer proposes and explains a decision, a typed Rust gate verifies it, and a fast reflex loop executes it without allowing slow inference to destabilize control.**

This is the part judges should evaluate first. The robotics, fly biomimicry, weather, agriculture, and TINA-X work are applications and research tracks built around the same thesis: **learned or sensed signals are not enough when a system must reason about constraints, dependencies, exceptions, and novel combinations of events.**

## Why this is a MeTTa/Hyperon project

Hyperon’s Atomspace is a dynamic metagraph for representing knowledge, while MeTTa combines functional, logical, and process-calculus ideas and operates by querying and rewriting Atomspaces [1]. TINA-X uses those properties directly rather than mentioning MeTTa as a decorative AI label.

The repository contains executable MeTTa rules and bridges for:

| MeTTa/Hyperon contribution | What is implemented in TINA-X | Why it matters |
|---|---|---|
| **Typed symbolic brain seam** | `rust-core` defines `SymbolicBrain`, `MettaQuery`, `MettaResult`, a production `SubprocessBrain`, and a `FakeBrain` test double. | Rust control logic can be tested independently of a moving Hyperon runtime while still driving real MeTTa end to end. |
| **Two-rate reasoning architecture** | MeTTa/Hyperon runs in a slow supervisory loop; the allocation-free Rust reflex loop remains separate. | A symbolic decision can be explainable without putting millisecond-to-second inference inside a high-rate control path. |
| **Rule-based verification** | Agriculture “treat/don’t treat,” hazard guidance, quorum/cross-inhibition, supervisor gates, and decision metadata are represented as symbolic rules. | The system can expose the rule and facts behind an action instead of returning only an opaque score. |
| **Atomspace scalability discipline** | Benchmarks and `HotWorkingSpace` enforce a small working space and discourage naive full-space scans. | TINA-X turns an observed Hyperon performance risk into an explicit architectural decision. |
| **Novel-event reasoning** | TINA-X loads infrastructure dependencies into a MeTTa Atomspace and forward-chains cascading failures from compound scenarios. | The demo shows why symbolic composition is useful for events that were not present as a training example. |
| **Cross-domain reuse** | The same symbolic pattern is applied to agriculture, weather guidance, swarm coordination, and infrastructure resilience. | The project demonstrates a reusable MeTTa reasoning substrate, not a single fly-only toy. |

The official Hyperon project describes the ecosystem as active pre-alpha software and experimentation [2]. TINA-X therefore labels what is measured, what is a proof of concept, and what remains aspirational instead of presenting the whole roadmap as production-ready.

## The judge-facing demo

The strongest hackathon story is a short, reproducible chain rather than a tour of every folder:

```text
sensor / scenario facts
        ↓
MeTTa rules in an Atomspace
        ↓
reasoned proposal + explanation
        ↓
Rust verification gate
        ↓
fast reflex / swarm / dashboard action
```

Run the symbolic smoke test first, then show the Rust bridge and the independent TINA-X cascade reasoner:

```bash
# MeTTa / Hyperon
python3 -m venv .venv
.venv/bin/pip install hyperon
.venv/bin/python metta-logic/run_smoke.py
.venv/bin/python metta-logic/bridge_worker.py '!(+ 1 2)'

# TINA-X: an independent MeTTa application
cd tina-x
python3 -m venv .venv-tina
.venv-tina/bin/pip install hyperon pytest
.venv-tina/bin/python -m tina_x.demo
.venv-tina/bin/python -m pytest
```

If Rust and Node are installed, run the complete software demo described in [`docs/DEV_SETUP.md`](./docs/DEV_SETUP.md). The frontend is a mission-control view of telemetry and slow-brain decisions; it is not the reasoning engine itself.

## What TINA-X is—and is not

| It is | It is not yet |
|---|---|
| A MeTTa/Hyperon reasoning and verification layer connected to Rust. | A flight-proven insect-scale hardware platform. |
| A two-rate architecture with measured reflex and MeTTa baselines. | Evidence that Hyperon is production-ready on constrained embedded hardware. |
| A headless self-organizing swarm logic prototype. | A validated outdoor swarm operating at agricultural scale. |
| A precision-agriculture decision prototype with verifiable treatment gates. | A deployed pesticide-spraying product or agronomic efficacy study. |
| A calibrated synthetic weather-nowcasting pipeline with symbolic guidance and thermodynamic denoising experiments. | A demonstrated 99% real-world severe-weather forecast system. |
| TINA-X, an independently runnable cascading-failure reasoner. | A complete national infrastructure digital twin. |
| GNSS and GEOGLOWS/EO bridge work for location and hazard context. | A finished Robonomics or Unity end-to-end deployment. |

## Research/application tracks

### 1. Precision agriculture

The primary application prototype uses symbolic constraints to decide whether a detected crop threat should be treated, withheld, or escalated. The important MeTTa question is not “can a model see a weed?” but “can the system justify an intervention under crop, confidence, safety, weather, and mission constraints?” See the perception pipeline in [`perception/`](./perception/) and the MeTTa agronomy rules in [`metta-logic/agronomy/`](./metta-logic/agronomy/).

### 2. Harsh-weather nowcasting

`weather-prediction/` combines a calibrated probabilistic baseline, symbolic hazard guidance, thermodynamic/p-bit denoising experiments, and distributed sensor/EO/GNSS inputs. Its README explicitly keeps the strongest claims honest: the unified stack, regional generalization, and real-world reliability remain open validation work.

### 3. TINA-X infrastructure resilience

[`tina-x/`](./tina-x/) is an **independent component**, not merely a fly application. It reasons over infrastructure dependencies—power, hospitals, roads, fuel, and other services—and derives cascading failures from compound “black swan” scenarios. It can optionally send alerts to the TINA-X dashboard, but it does not require TINA-X to run.

### 4. Self-organizing swarms

`rust-swarm/` implements headless swarm logic: neighbor-local agents, hierarchy/election, coverage, stigmergy, self-healing behavior, and a hazard overlay. It is the decentralized coordination application of the same reasoning architecture; Unity visualization and hardware integration remain later work.

### 5. Verification and provenance

`zk-rust/` contains a working proof/verification POC for a safe-to-fly decision. `zk-cairo/` is a dormant reference implementation. The ZK layer is a provenance and verification extension, not a claim that MeTTa rules are already proven on-chain.

## Architecture

```text
                    slow, explainable loop
┌──────────────────────────────────────────────────────────────────┐
│ MeTTa / Hyperon Atomspace                                        │
│ rules · dependencies · constraints · quorum · hazard guidance    │
└───────────────┬───────────────────────┬──────────────────────────┘
                │ proposal + explanation│ telemetry / facts
                ▼                       ▲
┌───────────────────────────┐   ┌─────────────────────────────────┐
│ Rust supervisory gate      │   │ applications                    │
│ verifies / rejects / holds │   │ agriculture · weather · TINA-X  │
└───────────────┬───────────┘   │ swarm · dashboard · ZK POC       │
                │ setpoint       └─────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────┐
│ fast Rust reflex loop: delayed-PD stabilization and actuation     │
└──────────────────────────────────────────────────────────────────┘
```

**Hard rule:** symbolic inference never runs inside the reflex path. The brain proposes; the gate verifies; the reflex executes.

## Repository map

```text
ROADMAP.md                    phased implementation plan
fly-biomimicry/               biological design evidence and citations
limitations-edge-cases/       constraints, risks, and mitigations
metta-logic/                  MeTTa rules, smoke tests, benchmarks, bridges
rust-core/                    reflex loop, brain bridge, supervisor, agriculture, telemetry
rust-swarm/                   headless SoNS coordination, coverage, stigmergy, GNSS, hazards
rust-server/                  Axum mission-control API and telemetry feeds
frontend/                     React dashboard and judge-facing telemetry view
weather-prediction/           calibrated nowcasting and symbolic guidance experiments
tina-x/                       independent MeTTa cascading-failure reasoner
zk-rust/                      working Rust Groth16 verification POC
zk-cairo/                     dormant Cairo reference implementation
unity-sim/                    simulation scaffold and Rust bridge contract
docs/                         setup, ADRs, benchmarks, and audit notes
```

## Hackathon verdict: promising, but not yet a winner by default

**TINA-X can be competitive in a MeTTa/Hyperon hackathon, but the current breadth is a liability unless the submission is framed around one undeniable symbolic demo.** The winning argument is not “we built autonomous fly swarms.” It is:

> **TINA-X demonstrates how MeTTa can sit above real-time agents as an auditable, reusable reasoning layer, and it proves the pattern across a safety gate, a novel infrastructure cascade, and decentralized coordination.**

The project currently earns credibility from executable rules, a Rust↔MeTTa bridge, benchmarks, tests, and clear limitations. It loses points when agriculture, weather, ZK, Unity, Robonomics, and fly biology appear as equal priorities. Judges should not have to infer the MeTTa contribution from a large repository.

Before submission, make the demo prove three things in under five minutes: **a rule fires**, **the explanation is visible**, and **the downstream action changes because of the symbolic result**. Use TINA-X as the clearest “novel composition” case and one agriculture or swarm gate as the embodied-action case. Put weather and the broader research paper in the appendix unless the event specifically asks for them.

See [`docs/HACKATHON_GRILL.md`](./docs/HACKATHON_GRILL.md) for the adversarial questions and a submission plan.

## Status and limitations

This is an open research prototype. Hyperon is an active pre-alpha ecosystem [2]. TINA-X’s own benchmarks are local measurements, not vendor guarantees; the MeTTa subprocess bridge is a deliberate Phase-0 integration seam; Unity, hardware, Robonomics, large-scale deployment, and field validation are not complete. The project is strongest when it shows the evidence and the boundary of each claim.

## References

[1]: https://hyperon.opencog.org/ "OpenCog Hyperon — Atomspace and MeTTa overview"
[2]: https://github.com/trueagi-io/hyperon-experimental "trueagi-io/hyperon-experimental — MeTTa implementation"
[3]: https://singularitynet.io/research/metta-programming-language/ "SingularityNET — MeTTa Programming Language"
[4]: https://www.blockchaincentrenbo.com/events/metta-training-hackathon-2025/ "MeTTa Training/Hackathon 2025 — developer tracks"

## Contributing and license

We welcome MeTTa developers, symbolic-AI researchers, Rust engineers, robotics and ML practitioners, weather scientists, and Web4 builders. Every new subsystem should identify the MeTTa rule, interface, benchmark, or application claim it adds. Project TINA-X is MIT licensed.
