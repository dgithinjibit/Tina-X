# Zero-Knowledge proofs in Project TINA-X

## Why ZK at all (read this first)
TINA-X is a **decentralized swarm** whose whole differentiator is **verifiable** decisions
(the MeTTa "verification moat"). ZK proofs make that verifiability *cryptographic and portable*:
one agent (or a blockchain) can trust another agent's decision **without re-running it and without
seeing its private internal state**.

We are **software-first**; hardware/sensors come in later phases. So the first proof is about a
*decision*, not a physical measurement.

## What is NOT worth proving (avoid crypto cargo-culting)
- The **reflex loop** — it's a microsecond control loop; a ZK proof takes ~seconds. Absurd mismatch.
- **ML inference** — proving real neural nets in ZK isn't practical yet. Not now.

## The first statement we prove — "Verifiable Safe-to-Fly Decision"
This is the exact, identical statement implemented by BOTH proof-of-concepts (Rust-native and
Cairo), so the comparison is apples-to-apples.

> **Claim:** *"Given a committed wind reading `w` and a committed tolerance `t`, the agent's
> decision `d` (1 = fly, 0 = refuse) is the CORRECT output of the safety rule
> `d = (w <= t) ? 1 : 0`."*

### Inputs
| name | role | why |
|---|---|---|
| `w` — wind reading (m/s) | **private** (witness) | we don't want to leak the raw environmental reading |
| `t` — wind tolerance (m/s) | **public** | the agent's published safety limit; anyone can check it |
| `d` — decision (0/1) | **public** | the action the agent claims it took |

### What a valid proof convinces a verifier of
- The agent applied the **real rule** (not a made-up one) to produce `d`.
- It did so for a wind value that is consistent with the claimed decision.
- All **without revealing `w`**.

This mirrors the MeTTa rule we already ship
(`metta-logic/smoke_test.metta`: `safe-to-fly? w = (<= w tol)`), so the ZK layer is proving the
SAME logic the symbolic brain runs — that is the point: cryptographically attest a MeTTa decision.

## Comparison criteria (decided in ADR 0004 after both POCs exist)
1. **Does it actually prove & verify?** (correctness roundtrip, rejects a lying prover)
2. **Proof / verify time** for this tiny circuit.
3. **Integration** with the rest of TINA-X (Rust core, Robonomics/on-chain path).
4. **Dependency & toolchain weight** (build time, extra runtimes).
5. **Ergonomics** for a junior dev to extend to the next statements.

## POCs
- Rust-native (arkworks Groth16/BN254): `zk-rust/` — `cargo run -p tina-zk-rust --release`
- Cairo (Scarb): `zk-cairo/` — `scarb cairo-test`

Both prove the statement above and reject a prover who claims a decision inconsistent with the rule.

## Outcome (see `docs/adr/0004-zk-stack.md`)
Measured: Rust Groth16 does a full prove (161 ms) + verify (3.5 ms) roundtrip in-workspace today;
Cairo implements the same rule with passing tests and is our Starknet-native on-chain target.
**Decision: build/iterate ZK in `zk-rust` (Rust, one toolchain), settle on-chain in Cairo/Starknet
later.** Next statement to prove: "the decision came from the agent's signed MeTTa ruleset."
