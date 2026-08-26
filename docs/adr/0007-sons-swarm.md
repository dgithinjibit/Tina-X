# ADR 0007 — SoNS swarm: gradient leader election, neighbor-local only

- **Status:** Accepted
- **Date:** 2026-08-08
- **Phase:** 4 (P4.1–P4.5)

## Context
Phase 4 scales one bot to a fleet. The roadmap requires a **Self-organizing Nervous System**: a
runtime-formed hierarchy with a transient, interchangeable "brain" that reconfigures on failure —
and it must be *verified* self-organization, not "emergent-and-hoped". The coordination logic is
fully buildable **headless** (Rust, no Unity); only the 3D view is deferred to Phase 7. The hard
constraint is that no agent may read global state — coordination must emerge from neighbor-local
messages alone, or it isn't a swarm.

## Decision
Model leader election as a **distributed max-consensus with distance** (`tina-swarm`):

- Each agent holds a `LeaderBelief { leader, distance }` and gossips it to neighbors each tick.
- On receipt, an agent adopts the best belief among {itself, neighbors' beliefs +1 hop}, where
  "best" = higher leader id, ties broken by shorter distance. The **highest agent id** thus becomes
  the brain — a stable, deterministic choice needing no shared clock or registry — and every agent
  learns its hop-distance to it (a gradient backbone, the "nervous system").
- Only a live agent that is its own local maximum (re)creates a `distance 0` source.
- **Stigmergy** (`CoverageField`): agents mark their current cell while it is under-covered; the
  mark IS the task allocation, so coverage spreads to gaps with no central scheduler.

### Self-healing + the count-to-infinity bound
Belief in a leader is sustained only by fresh gossip carrying its id from a distance-0 source. When
the brain dies it stops being that source; its former followers can only re-hear the id via
neighbors no closer than themselves, so the reported distance climbs every tick. A **freshness
horizon** (`sons::MAX_HORIZON`) rejects a belief once its distance exceeds it, so the stale belief
expires and survivors elect the next-highest id by the same rule — no failure detector, no election
message. This is the classic distance-vector "count to infinity", deliberately **bounded** by the
horizon. Consequence: recovery after leader loss is **O(horizon)** ticks — bounded and
deterministic. The horizon is set just above the largest graph diameter we run, so it never rejects
a belief from a *live* leader (whose gradient stays short).

## Consequences
- (+) Genuinely decentralized: enforced by module boundaries (an agent's `consume` only sees its
  inbox). Convergence is O(diameter), independent of agent count (verified at 400 agents).
- (+) Interchangeable brain + membership changes handled by one rule (kill/revive both work).
- (−) Leader-loss recovery is O(horizon), not O(diameter) — a bounded but non-instant cost. A
  strictly-closer-gradient variant could make it O(diameter); deferred as an optimization since the
  bounded behavior already meets the resilience exit criteria.
- (−) Fixed grid positions (no mobility) in Phase 4 — coordination/coverage don't need movement to
  be proven; mobility is a later concern.

## Validation
- `cargo test -p tina-swarm` — 12 unit + 7 emergent-behavior tests: single-leader convergence,
  distance gradient, brain-death re-election, higher-id rejoin, full-field stigmergic coverage,
  coverage-survives-partial-loss, 400-agent scale.
- `cargo run -p tina-swarm --bin swarm-demo` — self-organize → cover → kill brain → self-heal.
