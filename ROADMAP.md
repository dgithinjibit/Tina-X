# 🗺️ Project Nzi — Build Roadmap

> **Principle: one great bot before a fleet.** Every phase ends in something *runnable* and
> *measured*. We do NOT lock in architecture before we benchmark. Each phase raises the two
> feasibility numbers (maturity ~30–40%, arch-fit ~25–35%) with evidence, not optimism.

Legend: 🔴 not started · 🟡 in progress · 🟢 done · ⚠️ open research risk

---

## Phase 0 — Foundation & Truth-Finding (Weeks 1–2) — 🟢 MOSTLY DONE
*Goal: know what we're standing on before we build on it.*

- 🟢 **P0.1** Scaffold the repo: `rust-core/` cargo workspace, `metta-logic/`, `docs/`. *(done)*
- 🟢 **P0.2** Install & smoke-test Hyperon (0.2.10) in a venv; MeTTa runs from Python
  (`metta-logic/run_smoke.py` → nzi-001 / True / False). Bindings confirmed pybind11/C-API. *(done)*
- 🟢 **P0.3** ⚠️ **Benchmarked MeTTa ourselves** — parse-vs-eval split + Atomspace-size sweep
  (1e3/1e5/1e6). Findings in `docs/benchmarks/metta-baseline.md`:
  rule-eval ~2–7 ms p99 (size-independent, GOOD); direct `space.query()` is **O(n)**
  (~2 s @100k, ~23 s @1M) → forced **ADR 0002 (partitioned Atomspace)**. *(done)*
- 🔴 **P0.4** Call Rust `libhyperon` as a Cargo crate from a tiny Rust binary (prove Rust↔MeTTa).
  *(reflex loop + benchmark done in Rust; the Rust↔MeTTa crate bridge is the one open P0 item.)*
- 🟢 **P0.5** ADRs written: `0001-two-rate-brain.md`, `0002-partitioned-atomspace.md`. *(done)*

**Exit criteria:** MeTTa runs from Python ✅, real latency numbers recorded ✅, reflex loop
validated (~900× under budget) ✅. Remaining: Rust↔MeTTa crate bridge (P0.4).

---

## Phase 1 — One Simulated Bot: the Reflex Loop (Weeks 3–5)
*Goal: a single agent that flies/moves stably in simulation. No intelligence yet — just the fly's spine.*

- 🔴 **P1.1** Unity ML-Agents scene: one agent, physics, a few obstacles.
- 🔴 **P1.2** Rust reflex loop: **delayed-PD stabilizer** driven by simulated rate gyros,
  budgeted **<13 ms** (fly halteres spec). Runs as a fixed-rate control loop (>50 Hz).
- 🔴 **P1.3** Optic-flow-style obstacle avoidance (start crude — inter-sensor flow difference).
- 🔴 **P1.4** Collision *tolerance*: verify the agent recovers from bumps (crash-and-recover),
  don't over-engineer avoidance.
- 🔴 **P1.5** Telemetry stream out of the agent (feeds the symbolic brain in Phase 2).

**Exit criteria:** one agent holds attitude and navigates a cluttered scene reliably at >50 Hz,
reflex loop measured under 13 ms.

---

## Phase 2 — The Symbolic Brain + Verification Moat (Weeks 6–9)
*Goal: the slow MeTTa brain supervises the fast loop — and verifies its own decisions. This is the defensibility.*

- 🔴 **P2.1** MeTTa knowledge base in `metta-logic/`: world facts, agent state, mission goals as Atoms.
- 🔴 **P2.2** Slow supervisory loop: MeTTa reasons over telemetry → emits setpoints to the reflex
  loop (NEVER inside the <13 ms path).
- 🔴 **P2.3** ⚠️ **Verification loop** — the moat. Map the 5 agent-hallucination types
  (Reasoning/Execution/Perception/Memorization/Communication) to symbolic checks that gate actions.
  Output: `metta-logic/verification/` + doc mapping each type → check.
- 🔴 **P2.4** Governance: persistent workspace + skills; agent refuses/flags low-confidence actions
  (kill knowledge-boundary overconfidence).
- 🔴 **P2.5** Fault-injection tests: feed the agent bad/contradictory data, confirm verification catches it.
- 🔴 **P2.6** ⚠️ **Implement ADR 0002 (partitioned Atomspace)** — small hot working-space per agent +
  rule-driven inference over cold knowledge; re-benchmark to confirm query p99 no longer grows with
  total knowledge size. *(Required because P0.3 found `space.query()` is O(n).)*

**Exit criteria:** the agent explains and *symbolically justifies* every action; injected faults
are caught by the verification layer, not acted on.

---

## Phase 3 — First Real Task: Precision-Ag Perception (Weeks 10–13)
*Goal: the bot does something economically real — in sim first.*

- 🔴 **P3.1** Weed/pest detection model (Python/ML) on simulated crop imagery.
- 🔴 **P3.2** Sensor fusion (vision + context) feeding the MeTTa brain; symbolic rules decide
  "treat / don't treat" with justification.
- 🔴 **P3.3** Synthetic-data pipeline in Unity to fight the robotics data-scarcity bottleneck
  (generate long-tail cases).
- 🔴 **P3.4** Metrics: detection precision/recall + simulated pesticide-reduction %.

**Exit criteria:** one bot identifies weeds/pests in sim and makes verifiable treat decisions,
with a measured pesticide-reduction story.

---

## Phase 4 — From One to a Fleet: SoNS Swarm (Weeks 14–18)
*Goal: scale to a swarm using self-organizing hierarchy — only after ONE bot is solid.*

- 🔴 **P4.1** Multi-agent Unity scene; neighbor-local communication only.
- 🔴 **P4.2** Implement **SoNS** (Self-organizing Nervous System): runtime-formed hierarchy,
  transient interchangeable "brain" agent, reconfiguration on failure.
- 🔴 **P4.3** Stigmergy / task-allocation for coordinating precision-ag coverage.
- 🔴 **P4.4** Resilience tests: kill/add agents mid-mission; swarm must self-heal.
- 🔴 **P4.5** Scale test (sim): measure coordination overhead vs. agent count.

**Exit criteria:** a swarm covers a field, survives agent loss, and self-reorganizes — coordination
verified, not just emergent-and-hoped.

---

## Phase 5 — Weather Nowcasting Edge Case (Weeks 19–23)
*Goal: the flagship "wow." Distributed calibrated nowcasting — honest metrics, no 99% claims.*

- 🔴 **P5.1** Baseline: reproduce a WoFS-style ML hazard classifier on public data; report
  ETS / Brier / reliability (NOT accuracy).
- 🔴 **P5.2** Swarm-as-sensor-fleet: nodes contribute in-situ readings + spatiotemporal embeddings.
- 🔴 **P5.3** MeTTa symbolic constraint layer over ensemble outputs → interpretable hazard guidance.
- 🔴 **P5.4** ⚠️ **Explore** Extropic `thrml` for cheap probabilistic on-device sampling (aspirational).
- 🔴 **P5.5** Report lead-time & calibration improvements honestly.

**Exit criteria:** measurable improvement in calibrated hazard probability / lead time vs. baseline.

---

## Phase 6 — Decentralization & Hardware Path (Weeks 24+)
*Goal: real-world credibility.*

- 🔴 **P6.1** Robonomics integration: agent identity, signed telemetry, missions.
- 🔴 **P6.2** WASM/embedded target for `rust-core` (⚠️ build from scratch — no Hyperon support today).
- 🔴 **P6.3** ⚠️ Hardware spike: cheapest viable physical agent; confront the untethered-power problem
  head-on (insect-scale power is unsolved — pick a realistic node size).
- 🔴 **P6.4** Sim-to-real transfer trials.

---

## Cross-cutting tracks (run continuously)
- **Benchmarks** — keep `docs/benchmarks/` current; every real-time decision cites a number.
- **ADRs** — one architecture-decision record per major choice in `docs/adr/`.
- **Research folders** — grow `fly-biomimicry/`, `limitations-edge-cases/`, `weather-prediction/`
  as we learn; they're living design docs.
- **YC readiness** — after Phase 3, we have a demoable wedge that maps to YC's named ag-robotics RFS.

---

## Open research risks to retire (tracked, not hidden)
1. ⚠️ Hyperon is pre-alpha & unbenchmarked → **P0.3** retires this.
2. ⚠️ Symbolic reasoning latency vs. reflex loop → **P0.3 + P2.2** validate the split.
3. ⚠️ No Hyperon WASM/embedded/.NET path → **P6.2** builds it.
4. ⚠️ Untethered power at insect scale is unsolved → **P6.3** picks a realistic scale.
5. ⚠️ 99% weather reliability is not real → **P5** targets calibrated skill instead.

---

### Suggested first sprint (this week)
`P0.1` → `P0.2` → `P0.3`. If MeTTa latency is acceptable, proceed to Phase 1. If not, we redesign
the symbolic layer *before* writing agent code — cheaply, now, not later.
