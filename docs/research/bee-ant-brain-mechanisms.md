# Bee + Ant collective-decision mechanisms → TINA-X brain upgrades

> Deep-research run 2026-08-09 (105 agents, 23 sources, 24/25 claims adversarially confirmed).
> Scope: enrich the SLOW MeTTa brain + swarm layer with honeybee (quorum, cross-inhibition) and
> ant (stigmergy/ACO, response-thresholds) mechanisms. This doc is the design map we implement
> against; every mechanism names the existing TINA-X component it extends and whether it belongs in
> the FAST Rust loop or the SLOW MeTTa brain.

## The one-line split (design rule)
Any mechanism needing only a **local read/write + O(1) arithmetic** → **fast Rust** (`rust-swarm`).
Any mechanism needing **comparison across options, a policy/risk parameter, or a justification** →
**slow MeTTa brain** (`metta-logic/` via `rust-core::supervise`, never in the <13 ms reflex path).

---

## HONEYBEE mechanisms

### B1. Quorum-threshold commitment (SLOW / MeTTa) — *implemented*
Nest-site selection is evidence accumulation: scout populations accumulate noisy support for
competing sites and commit when one crosses a **quorum (~10–20 bees)**, triggering piping/takeoff.
Seeley & Visscher 2004 proved it's *quorum* (a local count), not *consensus*, via a dilution
experiment (5 boxes delayed piping from ~79→244 min).
- **Extends:** `metta-logic/knowledge/*.metta` + `rust-core::supervise` (slow loop), style mirrors
  `agronomy/treat.metta`'s single-place threshold.
- **Rule shape:** `support(option)` = count of agents committed; `committed?(o) = support(o) >= quorum`;
  `decide-site(options) = if committed?(best) then commit(best) else keep-scouting`.
- Accumulation (counts) rides fast Rust gossip; the threshold test + commit are slow-brain.

### B2. Quorum as a risk-tuned speed/accuracy knob (SLOW / MeTTa) — *implemented*
Small quorum → speed; large quorum → accuracy (Passino & Seeley 2006; optimal ~15–20).
- **Rule:** `quorum-threshold(risk high) = 20`, `quorum-threshold(risk low) = 6`. Irreversible acts
  (spraying, per `treat.metta` fail-closed) demand a **larger** quorum. Slow-brain policy param.

### B3. Cross-inhibitory STOP signal (FAST emit + SLOW arbitrate) — *implemented*
Each scout sends inhibitory stop signals **only to dancers advertising a *different* site**, which
breaks deadlock over equal options AND reduces "split" decisions **without impairing accuracy**
(Seeley et al. 2012; Laomettachit et al. 2016).
- **Extends:** `rust-swarm::sons` gossip (emit the cheap directed token) + slow MeTTa arbitration.
- **Rule shape:** `recruit-weight(o) = max(0, base-support(o) - inhibition-received(o))`;
  `inhibit(me, other) = if option(me) != option(other) then send-stop(other)`.
- Why it matters for TINA-X: current SoNS max-consensus (highest-id-wins) is deterministic but
  **arbitrary**; cross-inhibition adds a **value-sensitive** deadlock-breaker for equal-value targets.

### B4. Inhibition gain is a bifurcation parameter, coupled to quorum (SLOW / MeTTa) — *implemented*
Pitchfork bifurcation (Franci arXiv:1503.08526): below a critical cross-inhibition value → deadlock;
above → forced unanimity. But strong inhibition is **beneficial at LOW quorum, harmful at HIGH
quorum** (traps in local optima — Laomettachit 2016).
- **Rule:** `inhibition-gain(quorum q) = if q < 10 then high else off`. Strong inhibition for fast
  low-quorum tactical picks; rely on quorum alone for high-accuracy strategic picks. Prevents the
  swarm converging fast onto a wrong-but-locally-optimal field.

### B5. Quality×distance value (quality-dependent distance preference) (SLOW / MeTTa)
Site value trades quality against distance, and the distance preference **flips with quality**:
low-quality → moderate>near>far; high-quality → near>moderate>far (Laomettachit 2014, *model*).
- **Rule:** `site-value(s) = combine(quality(s), distance(s))` with `combine` quality-dependent.
  The bee analog of ACO's heuristic η and `treat.metta`'s confidence gate. *Model prediction — treat
  as directional.* (Deferred: implement once B1–B4 land and we have a real value signal.)

---

## ANT mechanisms

### A1. Pheromone deposit + EVAPORATION + τ^α selection (FAST / Rust) — *implemented*
ACO math (Dorigo & Stützle, Scholarpedia + monograph): transition
`p(c_ij) = τ_ij^α·η_ij^β / Σ τ_il^α·η_il^β`; update `τ_ij ← (1-ρ)·τ_ij + ρ·ΣF(s)`, ρ∈(0,1].
- **Extends:** `rust-swarm::stigmergy::CoverageField`, currently `cells: Vec<u32>` that ONLY
  increments (`mark` does `+=1`, never decays) and is read via `level()`.
- **Change:** (a) cells → `f64` pheromone; (b) `evaporate(rho)` applies `c ← (1-rho)*c` every tick;
  (c) `deposit(x,y,amount)`; (d) fast neighbor-selection weighted by `level^alpha` (× 1/distance η).
  Pure fast-loop math, no MeTTa.

### A2. Evaporation is "useful forgetting" — ESSENTIAL for adaptivity (FAST / Rust) — *implemented*
Scholarpedia: evaporation "favors exploration of new areas." S-ACO: with **ρ=0 the algorithm does
not converge** (stuck); ρ=0.01–0.1 works.
- **Direct TINA-X consequence:** our monotonic `CoverageField` **IS the broken ρ=0 case** — covered
  cells stay hot forever, so the field can't re-adapt to re-infestation, re-treatment need, or a
  downed agent's zone going stale. `evaporate(ρ)` makes coverage self-heal and re-flow to gaps —
  the mechanism behind our own P4.4 resilience goal. ρ is tuned (too high hurts convergence, too low
  locks in), NOT "higher is better."

### A3. Response-threshold task allocation (SLOW / MeTTa) — ⚠️ LOW CONFIDENCE, verify first
Fixed & reinforced-threshold division of labor (Bonabeau/Theraulaz/Deneubourg): an agent engages a
task when its stimulus exceeds the agent's threshold; reinforced thresholds fall with use
(specialization) and rise with disuse.
- **NOTE:** *No response-threshold claim survived the 3-vote adversarial check in this run.* Included
  as a synthesis recommendation only — **verify the primary sources before building.** Deferred.
- **Proposed shape (unverified):** `engage?(agent, task) = stimulus(task) >= task-threshold(agent, task)`;
  lower threshold after performing (specialize), raise when idle.

---

## Honest caveats (do NOT overclaim)
1. **Functional analogy, not identity** — the brain↔swarm framing is how the *authors* model it;
   don't market biological fidelity.
2. **Several results are model/simulation, not field data** — split-reduction, quality×distance
   reversal, pitchfork bifurcation, "natural-selection-tuned quorum." Numbers are directional.
3. **Bio numbers don't transfer directly** — quorum ~10–20 bees is a *ratio to steal*; TINA-X swarm
   sizes differ. ACO's ρ/α/β are hyperparameters with no biological value — **sweep them**.
4. **Where it breaks down (the one refuted claim, 0-3):** "stigmergy alone explains
   self-organization" is overstated — real self-organization needs the *full loop* (deposit +
   evaporation + probabilistic response + population dynamics), which is exactly why the mark-only
   `CoverageField` is incomplete. Drones also lack a persistent chemical medium; our shared coverage
   grid is a centralized-ish surrogate, not true environmental stigmergy.
5. **Response-thresholds (A3) unverified** — see above.

## Open questions (tracked)
- Real quorum threshold + inhibition gain for TINA-X's swarm sizes/risk levels → empirical sweep in the
  headless swarm before deployment.
- Can drones realize genuine stigmergy without a shared grid (RF beacons / visual markers / neighbor-
  local on-map pheromone)?
- Authoritative response-threshold formalization that survives verification (A3).
- How does value-sensitive cross-inhibition compose with SoNS max-consensus election — separate
  layers (election for backbone, quorum+inhibition for target choice) or a value-weighted election?

## Key sources
- Seeley et al. 2012, *Science* 335:108 — stop-signal cross-inhibition. https://www.science.org/doi/10.1126/science.1210361
- Seeley & Visscher 2004 — quorum sensing (dilution experiment). https://www.researchgate.net/publication/225491722
- Franci et al. arXiv:1503.08526 — pitchfork bifurcation of cross-inhibition. https://arxiv.org/pdf/1503.08526
- Laomettachit et al. 2016, *J Insect Behav* — split-decision reduction. https://link.springer.com/article/10.1007/s10905-016-9581-1
- Dorigo & Stützle — ACO. http://www.scholarpedia.org/article/Ant_colony_optimization + monograph.
