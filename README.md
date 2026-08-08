# 🪰 Project Nzi — Bio-Inspired Autonomous Swarms for a Better 2030

> **Nzi** means *fly* in Swahili. We borrow the fly's biology — its reflexes, its eyes,
> its resilience, its swarm behavior — to build cheap, decentralized robots that do
> tangible public good: **protecting crops, warning communities of harsh weather, and
> monitoring the living world.** Not war. A good life.

---

## 🌍 Vision

Nature already solved autonomy. A housefly stabilizes itself in **~13 ms** with no GPS, no
datacenter, and a brain smaller than a grain of rice — and flies coordinate as swarms with no
central controller. Project Nzi treats the fly as a *design spec*: a fleet of low-cost,
collision-tolerant agents, each running a fast biological reflex loop and a slow **symbolic
reasoning** brain (MeTTa/Hyperon), coordinating as a **self-organizing nervous system**.

The goal for 2030 isn't apocalypse-prevention theater — it's a **good life**: fewer pesticides
on our food, earlier warnings before a hailstorm hits a village, healthier ecosystems we can
actually see.

---

## 🎯 What Nzi does (focused, not everything-at-once)

**Primary wedge — Precision Agriculture Swarms.**
Bio-inspired agents that identify individual weeds and pests in real time and apply precise,
minimal treatment — cutting pesticide use while improving yield. *(This is a directly named
Y Combinator S26 funding priority, opened personally by Garry Tan.)*

**Flagship edge case — Harsh-Weather Nowcasting.**
The same swarm doubles as a distributed sensor fleet for **calibrated, energy-efficient
severe-weather prediction** (hail, severe convection) — improving *reliability and lead time*
so communities and the swarm itself can act before the storm. See
[`weather-prediction/`](./weather-prediction/).

**Secondary narrative — Environmental & ecosystem monitoring.**
Air/water/biodiversity sensing with the same cheap, resilient agents.

**Flagship reasoning application — [TINA-X](./tina-x/).**
A symbolic **cascading-failure reasoner**: a digital twin of society's fragility. It ingests
infrastructure dependencies (hospitals, grids, roads, fuel, data centers) into a MeTTa Atomspace
and *deduces* catastrophic cascades from compound "black swan" events it was never trained on
(earthquake → grid down → generator needs fuel → typhoon floods the road → hospital fails). This
is where symbolic AI beats deep learning. TINA-X is an **independent component** — it runs and is
tested standalone, and can optionally push alerts to the Nzi dashboard (see ADR 0005). Nzi's cheap
sensor-swarm becomes one of TINA-X's live data feeds.

---

## 🧬 The idea: borrow the fly's design

| Fly trait | What it gives us | Nzi design decision |
|---|---|---|
| Halteres (gyroscopic) | Attitude stability in ~5–13 ms | Fast rate-gyro **reflex loop** |
| PD sensorimotor control | Simple, robust low-level control | Delayed-PD stabilizer per agent |
| Compound-eye optic flow | GPS-free navigation | Lightweight optic-flow obstacle avoidance |
| Collision *tolerance* | Cheap survivable bodies | Crash-and-recover, obstacle-agnostic controllers |
| Swarm behavior | Decentralized coordination | **Self-organizing nervous system (SoNS)** |

Details & citations: [`fly-biomimicry/`](./fly-biomimicry/).

**The two-rate brain (core architectural principle):**
```
┌─ FAST reflex loop (<13 ms, on-agent) ──────────┐   ← fly halteres → PD control
│   rate gyros → PD stabilizer → actuators       │
└────────────────────────────────────────────────┘
            ▲ setpoints          │ telemetry
┌─ SLOW symbolic brain (MeTTa/Hyperon) ──────────┐   ← reasoning, verification, coordination
│   Atomspace reasoning + verification loop       │
└────────────────────────────────────────────────┘
```
> **Rule:** symbolic inference NEVER runs inside the reflex path. This is how we beat the
> ">50 Hz control vs. slow-model-inference" bottleneck. See [`limitations-edge-cases/`](./limitations-edge-cases/).

---

## 🧠 Why it can work (and where it's hard)

Autonomous agents are held back by **execution, not IQ**: CPU/tool latency (up to ~88% of
end-to-end), agent hallucinations, control-loop vs. inference latency, edge power limits, and
data scarcity. Our answer is **MeTTa's symbolic verifiability** as a governance layer over every
decision — that's the moat. The honest risks (unbenchmarked Hyperon, untethered insect-scale
power) are tracked openly in [`limitations-edge-cases/`](./limitations-edge-cases/).

**Feasibility (honest, self-assessed — see roadmap for how we raise these):**
- MeTTa/Hyperon ecosystem maturity today: **~30–40%** (pre-alpha, unbenchmarked)
- Architectural fit for swarm/real-time/embedded: **~25–35%** (much infra we build ourselves)

---

## 🛠️ Stack

- **Rust** — agent core, reflex loop, telemetry, WASM/embedded targets.
- **MeTTa / Hyperon (SingularityNET)** — symbolic reasoning, verification, swarm coordination.
  *Note: Python bindings are pybind11/C-API (not PyO3).*
- **Python** — ML training, sensor-fusion prototyping, `thrml`/probabilistic experiments.
- **Unity + ML-Agents** — simulation, sim-to-real, synthetic data (fights data scarcity).
- **Robonomics** — decentralized identity, telemetry, missions.
- **Extropic `thrml` (experimental)** — thermodynamic/probabilistic compute for cheap on-device
  inference (aspirational; see weather folder).

---

## 📂 Project Structure

```
project-nzi/
│
├── ROADMAP.md              # Phased build plan — START HERE to code
├── fly-biomimicry/         # Fly traits → Nzi design decisions (cited)
├── limitations-edge-cases/ # What holds agents back + our mitigations (cited)
├── weather-prediction/     # Harsh-weather nowcasting edge case (cited)
│
├── rust-core/              # agent core: reflex loop (fast) + MeTTa brain bridge (slow)
│   └── src/reflex.rs, brain.rs, roofline.rs, telemetry.rs   # + bins & integration tests
├── rust-server/            # mission-control API (axum): serves telemetry + BOM to the frontend
├── frontend/               # React + TypeScript landing / dashboard (Vite + Vitest)
├── zk-rust/                # ZK POC (arkworks Groth16): verifiable agent decisions
├── zk-cairo/               # ZK POC (Cairo/Scarb): same rule, Starknet-native on-chain target
├── tina-x/                 # 🌍 TINA-X: symbolic cascading-failure reasoner (INDEPENDENT component)
├── metta-logic/            # symbolic layer: smoke test, benchmark, Rust bridge worker
├── unity-sim/              # Phase 1 sim SCAFFOLD (C# ML-Agents + Rust bridge contract; build in Unity)
├── robonomics-integration/ # (to be built, Phase 6) identity, telemetry, missions
└── docs/                   # ADRs (0001–0003), benchmarks, scaling/, DEV_SETUP
```

---

## 🚀 Getting Started

See **[`ROADMAP.md`](./ROADMAP.md)** for the phased plan and **[`docs/DEV_SETUP.md`](./docs/DEV_SETUP.md)**
for exact commands. Quick taste:
```bash
rustup default stable                 # Rust toolchain
cargo test                            # 48 Rust tests (core + server + ZK + bridge integration)
cargo run -p nzi-server               # mission-control API on http://127.0.0.1:8080
cd frontend && npm install && npm run dev   # live dashboard (landing + telemetry + BOM)
```

### 🖥️ Mission control (for hackathon judges / YC)
Because Nzi is an IoT project, the **[`frontend/`](./frontend/)** React app is a live,
verifiable landing page: real reflex telemetry streamed from Rust, the two-rate-brain
explanation, and the **bill of materials** for one agent. Start `nzi-server`, then the frontend.

---

## 🤝 Contributing

We want **Rust engineers, robotics/AI researchers, ML/weather folks, and Web4 builders.**
Every subsystem should map to a design decision in one of the three research folders. Open an
issue or PR.

## 📜 License

MIT — open to contributors.

---

*Research foundation: all claims in the three research folders are adversarially fact-checked and
cited. Nzi is honest about what's proven, what's aspirational, and what we have to build ourselves.*
