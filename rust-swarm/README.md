# `rust-swarm/` — headless SoNS swarm (Phase 4)

Scaling from one bot to a fleet using a **Self-organizing Nervous System (SoNS)**: a hierarchy
that forms at runtime from purely **neighbor-local** interactions, with an interchangeable "brain"
(leader) that is re-elected automatically when an agent fails. This is the coordination **logic**,
built and proven **headless** — no Unity. Only the 3D visualization of the swarm is deferred to
Phase 7.

## The one invariant
**No agent ever reads global state.** Every decision uses only the agent's own state + messages
from its immediate neighbors. `Swarm::step` enforces this: it collects each agent's messages
(produced from own state), delivers them along topology edges only, then lets each agent consume
its inbox. The whole-swarm views (`leaders()`, coverage fraction) are for tests/telemetry, never
for an agent's decision.

## Layers (`src/`)
- `sons.rs` — `LeaderBelief` + the election rule: distributed **max-consensus with distance**. The
  highest agent id becomes the brain; each agent also learns its hop-distance to it (a gradient
  backbone). A freshness horizon (`MAX_HORIZON`) bounds the classic count-to-infinity so a dead
  leader's belief expires and a survivor takes over.
- `agent.rs` — an agent, its neighbor-local `produce`/`consume`, and the stigmergy coverage rule.
- `stigmergy.rs` — the shared `CoverageField` agents mark locally to allocate field coverage.
- `lib.rs` — the `Swarm` orchestrator (grid topology, lockstep message delivery).

## Emergent properties (proven in `tests/swarm_behavior.rs`)
- **P4.1/P4.2** a connected grid converges to exactly one leader (highest id) + a distance gradient.
- **P4.4** killing the brain → automatic re-election of the next-highest survivor (self-heal);
  a rejoining higher id resumes leadership; coverage laid before a partial loss is not undone.
- **P4.3** stigmergy covers the whole field and stops over-marking covered cells (no runaway spray).
- **P4.5** a 400-agent (20×20) swarm still converges to one leader; convergence is O(diameter),
  not O(agent-count) — the signature of neighbor-local coordination.

Recovery after leader loss is **O(horizon)** — a bounded, deterministic cost (the same technique
distance-vector routing protocols use to bound count-to-infinity). See `docs/adr/0007-sons-swarm.md`.

## Run
```bash
cargo run -p nzi-swarm --bin swarm-demo   # self-organize -> cover -> kill brain -> self-heal
cargo test -p nzi-swarm
```
