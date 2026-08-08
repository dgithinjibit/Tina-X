# Limitations & Edge Cases — What's Holding Autonomous Agents Back

> This folder exists because **Project Nzi's defensibility is its answer to these bottlenecks.**
> Every subsystem we build should map to a named limitation below and state how Nzi mitigates it.
> Claims here are backed by cited, adversarially-verified research (deep-research run, 2026-08-08).

The headline finding: **autonomous agents are not primarily bottlenecked by model IQ — they are bottlenecked by *execution*: latency, reliability, energy, data, and coordination.** That is good news for a symbolic-reasoning + swarm project like Nzi, because those are systems problems we can attack directly.

---

## 1. CPU / tool-execution latency (not GPU compute)
Tool processing on CPUs dominates end-to-end latency, consuming **up to ~88% of E2E latency** (83–89% for RAG retrieval, 85–88% for ChemCrow conformer generation). A stronger GPU just *shifts the bottleneck to the CPU*.
- Source: arXiv 2511.00739 (Georgia Tech + Intel, Nov 2025) — https://arxiv.org/html/2511.00739v3
- **Nzi implication:** MeTTa queries + sensor-fusion pipelines ARE the CPU-side work that will dominate latency. Profile and co-design the tool layer; do not assume a bigger model/GPU fixes real-time.

## 2. Agent hallucinations (5 structured types) + knowledge-boundary overconfidence
Distinct from LLM linguistic hallucinations: fabricated human-like behaviors from overconfidence, in five types — **Reasoning, Execution, Perception, Memorization, Communication.** A key failure is knowledge-boundary overconfidence: "answers that sound certain but are actually incorrect."
- Sources: arXiv 2509.18970 — https://arxiv.org/html/2509.18970v1 ; arXiv 2606.14502 — https://arxiv.org/pdf/2606.14502
- **Nzi implication:** map each hallucination type to an explicit mitigation. MeTTa's symbolic verifiability is our differentiator for the verification/governance layer. Reliable systems need "persistent Workspaces, skills, verification loops, and governance."

## 3. Latency vs. control-frequency mismatch
Robot control loops typically run **>50 Hz** for stability, but foundation-model inference adds **hundreds of ms to multiple seconds** — making FM-in-the-loop control inapplicable or unstable.
- Source: arXiv 2604.15395 — https://arxiv.org/pdf/2604.15395
- **Nzi implication:** hierarchical control — fast reflex loop on-agent (see fly biomimicry: <13 ms PD stabilizer), slow symbolic/cloud supervision. NEVER put symbolic inference inside the reflex path.

## 4. Edge SWaP-C / energy / thermal limits
Onboard platforms have strict size, weight, power, cost, and thermal constraints that prohibit hosting full-scale best-performing robotic foundation models (unlike the cloud).
- Source: arXiv 2604.15395 — https://arxiv.org/pdf/2604.15395
- **Nzi implication:** a fly-scale node can't host a big FM. Use small distilled/symbolic on-board reasoning; consider probabilistic/thermodynamic compute for cheap on-device inference (see ../weather-prediction/).

## 5. Robot data scarcity / long-tail
High-quality real-world trajectory data needs physical hardware, teleoperation, and time; rare long-tail events are especially hard to capture. Called "the most fundamental bottleneck of the field."
- Source: arXiv 2604.15395 — https://arxiv.org/pdf/2604.15395
- **Nzi implication:** lean on simulation (Unity ML-Agents) + synthetic data to fight the gap and sim-to-real; the swarm itself becomes a distributed data-collection fleet.

---

## How Nzi turns each limitation into a feature
| Limitation | Nzi mitigation |
|---|---|
| CPU/tool latency | Profile-first tool layer; MeTTa/MORK benchmarked before real-time lock-in |
| Hallucination (5 types) | Symbolic MeTTa verification loop over every agent decision (the moat) |
| >50 Hz vs FM latency | Fast fly-style reflex loop + slow symbolic supervisor (two-rate control) |
| Edge SWaP-C | Insect-scale, collision-tolerant cheap agents; probabilistic on-device inference |
| Data scarcity | Unity sim + swarm-as-sensor-fleet generating real long-tail data |
| Coordination | SoNS self-organizing hierarchy + stigmergy (see ../docs / architecture) |
