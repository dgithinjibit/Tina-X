# zk-cairo — DORMANT reference mirror (frozen 2026-08-09)

> **Status: dormant / not maintained.** This crate is a frozen reference implementation of the
> TINA-X "Verifiable Safe-to-Fly Decision" statement in Cairo (Starknet-native, STARK, no trusted
> setup). It is kept for reference only.

**The live ZK path is [`../zk-rust/`](../zk-rust/)** (arkworks Groth16/BN254), which proves and
verifies end-to-end today (setup 431 ms / prove 161 ms / verify 3.5 ms). Per the **ADR 0004
addendum** (`../docs/adr/0004-zk-stack.md`), all ZK iteration happens in Rust, and on-chain
settlement (P6.2) goes through a Rust/EVM-friendly BN254 verifier — **not** Starknet.

This mirror is intentionally *not* kept in lock-step with `zk-rust`; expect it to drift. It is
revived only if a Starknet-native settlement requirement appears — in which case start from this
code and re-sync it against the current `zk-rust` statement.

Original tests still run: `scarb cairo-test` (Scarb ~2.9.2). See the ADR for the full comparison
table and the reasoning behind making Rust primary.
