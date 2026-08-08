# 🗺️ Project Nzi — Build Roadmap

> **Principle: one great bot before a fleet.** Every phase ends in something *runnable* and
> *measured*. We do NOT lock in architecture before we benchmark. Each phase raises the two
> feasibility numbers (maturity ~30–40%, arch-fit ~25–35%) with evidence, not optimism.

Legend: 🔴 not started · 🟡 in progress · 🟢 done · ⚠️ open research risk · ⏭️ deferred (needs a tool outside this dev env)

> **Build-here-first pivot (2026-08-08).** We code everything that runs in this environment now,
> and defer ONLY what genuinely needs another tool. Two things move to the final **Phase 7 —
> External Runtimes**: (1) anything needing the **Unity engine** (the sim scenes), and (2)
> **physical hardware**. Everything else stays in place — including **on-chain work on a
> testnet** (reachable now) and **external live-API** integrations (kept optional/mockable so
> tests never require the internet). Deferred items are tagged ⏭️ and cross-linked to Phase 7.

---

## Phase 0 — Foundation & Truth-Finding (Weeks 1–2) — 🟢 DONE
*Goal: know what we're standing on before we build on it.*

- 🟢 **P0.1** Scaffold the repo: `rust-core/` cargo workspace, `metta-logic/`, `docs/`. *(done)*
- 🟢 **P0.2** Install & smoke-test Hyperon (0.2.10) in a venv; MeTTa runs from Python
  (`metta-logic/run_smoke.py` → nzi-001 / True / False). Bindings confirmed pybind11/C-API. *(done)*
- 🟢 **P0.3** ⚠️ **Benchmarked MeTTa ourselves** — parse-vs-eval split + Atomspace-size sweep
  (1e3/1e5/1e6). Findings in `docs/benchmarks/metta-baseline.md`:
  rule-eval ~2–7 ms p99 (size-independent, GOOD); direct `space.query()` is **O(n)**
  (~2 s @100k, ~23 s @1M) → forced **ADR 0002 (partitioned Atomspace)**. *(done)*
- 🟢 **P0.4** ✅ **Rust↔MeTTa bridge working.** Found `hyperon` is NOT on crates.io (git-only,
  no linkable `libhyperon`), so per **ADR 0003** we built a trait seam (`brain::SymbolicBrain`)
  with a `SubprocessBrain` that drives MeTTa via `metta-logic/bridge_worker.py`, plus a `FakeBrain`
  test double. `cargo run -p nzi-core --bin brain-demo` shows Rust getting typed results from
  MeTTa (`!(+ 1 2)` → `3`; safe-to-fly rule → `True/False`). Native FFI drops into the same trait
  later. *(done)*
- 🟢 **P0.5** ADRs written: `0001-two-rate-brain.md`, `0002-partitioned-atomspace.md`,
  `0003-rust-metta-bridge.md`. *(done)*

**Exit criteria — ALL MET:** MeTTa runs from Python ✅; real latency numbers recorded ✅; reflex
loop validated (~900× under budget) ✅; **Rust drives MeTTa end-to-end** ✅ (10 tests pass).
→ Ready for Phase 1 (Unity + real plant dynamics).

---

## Phase 1 — One Simulated Bot: the Reflex Loop (Weeks 3–5)
*Goal: a single agent that flies/moves stably in simulation. No intelligence yet — just the fly's spine.*

> **Build-here split:** the control side (fully testable here) is built and green in Rust. The
> parts that need the **Unity engine** to be meaningful (a real physics plant + obstacles) are
> deferred to **Phase 7**; a scaffold to open locally already exists in `unity-sim/`.

- ⏭️ **P1.1 → Phase 7** Unity ML-Agents scene: one agent, physics, a few obstacles.
  *Scaffolded in `unity-sim/` (C# `NziAgent`/`ReflexBridgeClient` + `SCENE_SETUP.md`); needs the
  Unity editor, so it's built in Phase 7.*
- 🟢 **P1.2** Rust reflex loop: **3-axis delayed-PD stabilizer** (`reflex::AttitudeStabilizer`,
  roll/pitch/yaw) driven by simulated rate gyros, budgeted **<13 ms** (fly halteres spec), running
  at 500 Hz. Unity↔Rust bridge contract (`unity_bridge.rs` + `unity-sim/BRIDGE_CONTRACT.md`) is
  defined and unit-tested end-to-end (reading → stabilizer → command converges on all axes). *(done)*
- ⏭️ **P1.3 → Phase 7** Optic-flow obstacle avoidance *against the Unity scene*. (A crude
  Rust-side optic-flow *model* could be prototyped here first if we want; the real validation
  needs the 3D scene.)
- ⏭️ **P1.4 → Phase 7** Collision *tolerance* (crash-and-recover) — needs physical contacts from
  the Unity physics engine to be meaningful.
- 🟢 **P1.5** Telemetry stream out of the agent (feeds the symbolic brain in Phase 2).
  *3-axis telemetry (`AttitudeSample`) streams from the sim to the dashboard (per-axis charts +
  brain-decision panel). Sufficient to drive Phase 2; richer obstacle/nav telemetry lands with
  the Unity scene in Phase 7.*

**Exit criteria (as achievable here):** the Rust reflex loop holds attitude on all three axes
from a disturbed start, at 500 Hz, measured under 13 ms, with telemetry streaming — ✅ **met**.
Navigation-in-clutter is validated in Phase 7 with the Unity plant.

---

## Phase 2 — The Symbolic Brain + Verification Moat (Weeks 6–9) — 🟡 ACTIVE (Stages ①② 🟢 done → Stage ③ next)
*Goal: the slow MeTTa brain supervises the fast loop — and verifies its own decisions. This is the defensibility. **100% buildable here.***

> **Runtime pattern (Rust-first, safety-critical in Rust).** The reasoning RULES live in `.metta`
> files; **Rust orchestrates and gates** every decision via the existing `brain::SymbolicBrain`
> seam (`SubprocessBrain` over a venv, `FakeBrain` for tests — ADR 0003). The MeTTa *engine* is
> Python-bound (no linkable `libhyperon`), but the safety-critical action-gate stays in Rust.
> Everything here runs and tests in this environment.

> **Waterfall order (do these in sequence):** ① Verification moat → ② Supervisory loop →
> ③ Partitioned Atomspace. Each stage lands green before the next starts.

**Stage ① — Verification moat (the defensibility; build FIRST). — 🟢 DONE**
- 🟢 **P2.1** ⚠️ **Verification rules** — the 5 agent-hallucination types
  (Reasoning / Execution / Perception / Memorization / Communication) are mapped to symbolic
  checks in `metta-logic/verification/*.metta`, one file per type, plus `limits.metta` (shared
  constants) and `verify.metta` (the `gate-setpoint` composer that returns
  `Approved` / `(Rejected <check>)`). `python3 metta-logic/run_verify_smoke.py` is green (6/6:
  one good action approved, one bad per type rejected with the right reason). *(done)*
- 🟢 **P2.2** **Rust action-gate** — `nzi-core::verify` (`Gate<B: SymbolicBrain>`, `Setpoint`,
  `Context`, `Verdict`) loads the `.metta` rules at runtime (single source of truth on disk),
  builds `<rules>\n!(gate-setpoint …)`, runs it through the `SymbolicBrain` seam and parses the
  verdict **fail-closed** (an unrecognized answer is never treated as approval). `FakeBrain`
  unit tests + `cargo run -p nzi-core --bin verify-demo`. *(done)*
- 🟢 **P2.3** **Fault-injection tests** — `rust-core/tests/verify_integration.rs` feeds the real
  gate a bad action per type and asserts each hallucination type is caught, not acted on
  (7 tests, venv-backed, skip-gracefully). `Verdict::is_approved()` maps directly onto the
  `BrainDecision.verified` wire flag for dashboard surfacing (Stage ② wires the live stream). *(done)*

**Stage ② — Supervisory loop (wire brain → reflex). — 🟢 DONE** *(ADR 0006)*
- 🟢 **P2.4** MeTTa knowledge base in `metta-logic/knowledge/`: `world.metta` (envelope + env
  facts), `mission.metta` (per-mode goal + the `decide-setpoint` reasoning rule), `agent.metta`
  (self identity/confidence). Proposals are clamped into the envelope so a healthy one is
  gate-consistent by construction. `python3 metta-logic/run_supervise_smoke.py` green (2/2). *(done)*
- 🟢 **P2.5** Slow supervisory loop: `nzi-core::supervise::Supervisor<B: SymbolicBrain>`. Each
  `tick` asks the brain to propose a setpoint (KB + telemetry), runs it through the Stage-① gate,
  and forwards ONLY approved setpoints to the reflex loop — the gate sits between the two rates,
  never inside the <13 ms path. Emits a `BrainDecision` per tick. `FakeBrain` unit tests +
  venv-backed `supervise_integration.rs` + `cargo run -p nzi-core --bin supervise-demo`. *(done)*
- 🟢 **P2.6** Governance (**fail-closed**): the supervisor retains the last-known-safe setpoint and
  holds it on any non-approval — `Refused(<check>)` (gate rejected) or `HeldOnDoubt(<reason>)`
  (empty/unparseable/errored proposal). Doubt is never treated as approval. Proven by the
  stale-telemetry integration test (refused → last-safe held). *(done)*

**Stage ③ — Partitioned Atomspace (retire the O(n) risk).**
- 🔴 **P2.7** ⚠️ **Implement ADR 0002** — small hot working-space per agent + rule-driven inference
  over cold knowledge; re-benchmark to confirm query p99 no longer grows with total knowledge
  size. *(Required because P0.3 found `space.query()` is O(n).)*

**Exit criteria:** the agent explains and *symbolically justifies* every action; injected faults
are caught by the verification layer, not acted on; query p99 is flat vs. knowledge size.

---

## Phase 3 — First Real Task: Precision-Ag Perception (Weeks 10–13)
*Goal: the bot does something economically real — in sim first.*

- 🔴 **P3.1** Weed/pest detection model (Python/ML) on simulated crop imagery.
- 🔴 **P3.2** Sensor fusion (vision + context) feeding the MeTTa brain; symbolic rules decide
  "treat / don't treat" with justification.
- ⏭️ **P3.3 → Phase 7** Synthetic-data pipeline in Unity to fight the robotics data-scarcity
  bottleneck (generate long-tail cases). *Needs the Unity engine; the detection model + symbolic
  treat/don't-treat logic (P3.1/P3.2/P3.4) can use public/recorded imagery here without it.*
- 🔴 **P3.4** Metrics: detection precision/recall + simulated pesticide-reduction %.

**Exit criteria:** one bot identifies weeds/pests in sim and makes verifiable treat decisions,
with a measured pesticide-reduction story.

---

## Phase 4 — From One to a Fleet: SoNS Swarm (Weeks 14–18)
*Goal: scale to a swarm using self-organizing hierarchy — only after ONE bot is solid.*

> The SoNS coordination LOGIC is buildable here as a headless multi-agent simulation in Rust
> (N `AgentSim`s + neighbor-local message passing) — no Unity needed to prove self-organization.
> Only the 3D visualization of the swarm is deferred.

- 🔴 **P4.1** Headless multi-agent sim in Rust: N agents, neighbor-local communication only.
  *(⏭️ the Unity 3D **visualization** of the scene → Phase 7; the coordination logic is here.)*
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

## Phase 6 — Decentralization (Weeks 24+)
*Goal: real-world credibility — **on-chain work stays here** (testnet is reachable now).*

- 🔴 **P6.1** Robonomics integration: agent identity, signed telemetry, missions — on **testnet**.
- 🔴 **P6.2** Deploy the ZK verifier on **Starknet testnet** (the local `zk-cairo` prove/verify
  already works; testnet deploy is reachable from here, so it is NOT deferred).
- 🔴 **P6.3** WASM target for `rust-core` (⚠️ build from scratch — no Hyperon support today). WASM
  builds and runs in this environment, so it stays here; the *embedded/on-metal* target is P7.

---

## Phase 7 — External Runtimes (LAST — needs a tool outside this dev env)
*Goal: everything that genuinely requires the **Unity engine** or **physical hardware**. Nothing
here blocks the software story; each item has a scaffold or a headless equivalent built earlier.*

- ⏭️ **P7.1** Build the Unity ML-Agents scenes from the `unity-sim/` scaffold and run the reflex
  loop against real physics (closes **P1.1**).
- ⏭️ **P7.2** Optic-flow obstacle avoidance + collision-tolerance in the 3D scene (closes **P1.3/P1.4**).
- ⏭️ **P7.3** Unity synthetic-data pipeline for long-tail perception cases (closes **P3.3**).
- ⏭️ **P7.4** Unity 3D visualization of the SoNS swarm (the logic is already headless in Phase 4).
- ⏭️ **P7.5** ⚠️ Hardware spike: cheapest viable physical agent; confront untethered insect-scale
  power head-on. **Embedded** target for `rust-core` (on-metal, beyond the WASM build in P6.3).
- ⏭️ **P7.6** Sim-to-real transfer trials.

**Exit criteria:** the proven-in-software system runs on a real 3D plant and a first physical node.

---

## Cross-cutting tracks (run continuously)
- **Benchmarks** — keep `docs/benchmarks/` current; every real-time decision cites a number.
- **Roofline/profiling** — 🟢 `rust-core/src/roofline.rs` + `docs/scaling/` (transferable ideas
  from jax-ml/scaling-book: compute-bound vs memory-bound reasoning). Apply it to new hot paths.
- **ADRs** — one architecture-decision record per major choice in `docs/adr/`.
- **Research folders** — grow `fly-biomimicry/`, `limitations-edge-cases/`, `weather-prediction/`
  as we learn; they're living design docs.
- **Mission-control demo** — 🟢 `rust-server/` (axum telemetry+BOM API) + `frontend/` (React/TS
  landing & dashboard). Grows every phase; this is the live artifact judges/YC open. Wire each new
  capability (brain decisions, weather nowcasts, swarm view) into it as it lands.
- **ZK verifiable decisions** — 🟢 foundation set: `zk-rust/` (arkworks Groth16, working
  prove/verify) + `zk-cairo/` (Starknet-native mirror). Per ADR 0004: build in Rust, settle
  on-chain in Cairo later. On-chain settlement targets a **testnet** (reachable now — NOT deferred
  to Phase 7). Next statement: prove a decision came from the signed MeTTa ruleset; then deploy the
  verifier to Starknet testnet (P6.2) and attach proofs to on-chain telemetry (P6.1 Robonomics).
- **TINA-X (flagship reasoning app)** — 🟢 core built: `tina-x/` independent component
  (ADR 0005). MeTTa dependency graph + cascading-failure rules + Python black-swan injector +
  optional Nzi-dashboard bridge (verified end-to-end). Next: OSM ingestion (real region),
  live API feeds (USGS/NOAA/DSCOVR), supply-chain + space-weather modules, and ZK-attested alerts.
  Grows independently of the robotics phases; Nzi's swarm becomes one of its sensor feeds.
- **YC readiness** — after Phase 3, we have a demoable wedge that maps to YC's named ag-robotics RFS.

---

## Open research risks to retire (tracked, not hidden)
1. ⚠️ Hyperon is pre-alpha & unbenchmarked → **P0.3** retires this.
2. ⚠️ Symbolic reasoning latency vs. reflex loop → **P0.3 + P2.2** validate the split.
3. ⚠️ No Hyperon WASM/embedded/.NET path → **P6.3** builds the WASM target here; **P7.5** the embedded one.
4. ⚠️ Untethered power at insect scale is unsolved → **P7.5** picks a realistic scale.
5. ⚠️ 99% weather reliability is not real → **P5** targets calibrated skill instead.

---

### Suggested first sprint (this week)
`P0.1` → `P0.2` → `P0.3`. If MeTTa latency is acceptable, proceed to Phase 1. If not, we redesign
the symbolic layer *before* writing agent code — cheaply, now, not later.
