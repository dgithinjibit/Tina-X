# ADR 0004 — ZK stack: Rust-native (arkworks) as primary, Cairo/Starknet as the on-chain target

- **Status:** Accepted
- **Date:** 2026-08-08
- **Phase:** ZK foundation (pre–Phase 1)

## Context
We need verifiable agent decisions (see `docs/zk/`). We built the SAME statement — the
"Verifiable Safe-to-Fly Decision" (`decision = 1 iff wind <= tolerance`, wind private) — two ways
and compared them head-to-head.

### The two POCs (both real, both tested)
| | Rust-native (`zk-rust/`) | Cairo (`zk-cairo/`) |
|---|---|---|
| Proving system | Groth16 zk-SNARK over BN254 (arkworks) | STARK (Cairo is provable-by-design) |
| Real proof today? | ✅ full prove+verify roundtrip in-process | ⚠️ logic + tests; prover/on-chain wiring is a follow-up |
| Measured perf | setup 431 ms, **prove 161 ms, verify 3.5 ms** | tests pass; gas est. ~800–1200 per call |
| Hides private `wind`? | ✅ verifier sees only (tolerance, decision) | ✅ by design at the prover boundary |
| Lying prover blocked? | ✅ cannot produce a valid proof | ✅ validator rejects inconsistent decision |
| Language / integration | Same Rust as `tina-core`; one toolchain | Separate language + 35 MB Scarb toolchain |
| Tests | 5 unit + doctest (`cargo test -p tina-zk-rust`) | 5 (`scarb cairo-test`) |
| On-chain / Robonomics fit | needs a verifier contract (BN254 verify is cheap & EVM-friendly) | **native** to Starknet |
| Trusted setup | ⚠️ yes (Groth16 per-circuit) | ✅ none (STARK) |

## Decision
**Use the Rust-native arkworks stack as the PRIMARY development path, and keep Cairo/Starknet as
the on-chain settlement target.** Concretely:

1. **Prototype & iterate ZK statements in `zk-rust`** — it's in our Cargo workspace, one language,
   proves+verifies end-to-end today, and a junior dev can extend a circuit without a second
   toolchain. Fast inner loop.
2. **Cairo/Starknet is where proofs ultimately settle** when we go on-chain (it's STARK-native,
   no trusted setup, and Robonomics/Web4 story benefits). Keep `zk-cairo` alive as the mirror of
   the same logic so the port is mechanical when we need it.

This honors the user's guidance ("build a POC in both, pick what works best, don't care which
language as long as the project runs, software-first"): Rust "works best" for *building now*;
Cairo wins for *on-chain later*. We don't have to choose one forever — we sequence them.

## Consequences
- (+) ZK development stays in the workspace (`cargo test` covers it); tight loop with `tina-core`.
- (+) Groth16's ~3.5 ms verify is ideal for a cheap on-chain/edge verifier.
- (−) Groth16 needs a per-circuit trusted setup — acceptable for a POC; revisit (PLONK/STARK) if
  circuits change often or trusted setup becomes a liability.
- (−) We maintain the logic in two places (Rust + Cairo) until the on-chain port. Mitigation: both
  are tiny and test the identical rule; a drift test can be added when it matters.

## Validation
- `cargo run -p tina-zk-rust --release` prints prove/verify timings and a valid=true roundtrip.
- `scarb cairo-test` (in `zk-cairo/`) passes the same rule's tests.
- Next ZK statement (e.g. "decision came from the signed MeTTa ruleset") is prototyped in
  `zk-rust` first, per this ADR.

## Addendum — 2026-08-09 (Phase 6 kickoff): Cairo goes DORMANT
`zk-rust` is proven, fast, and verifying end-to-end, so we commit to it as the sole ZK path we
build on. **We no longer maintain `zk-cairo` as a live mirror.** It stays in-tree as a reference
implementation of the same statement, but is **deferred/optional**: we will not invest in it,
will not keep it in lock-step with `zk-rust`, and will NOT block on Starknet-native on-chain
settlement. On-chain settlement (when we do it) goes through a Rust/EVM-friendly BN254 verifier
route instead. This is reversible — if a Starknet-native settlement requirement appears, we
re-activate `zk-cairo` from this frozen reference. Supersedes the "keep the mirror alive so the
port is mechanical" stance in the Decision above. ROADMAP P6.2 updated to match.
