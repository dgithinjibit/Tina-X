# 🎮 unity-sim — Phase 1 simulation (scaffold)

Unity ML-Agents scene for **one** Nzi agent: physics + rendering + obstacles. This is the
"real plant" the Rust reflex loop stabilizes against, replacing the toy rate-integrator in
`rust-server/src/sim.rs`.

> ⚠️ **This is scaffolding, not compiled here.** Unity + C# cannot build in the dev container
> this repo was authored in, so these scripts are written to be *dropped into a Unity project*
> and run locally. They are heavily commented for a junior dev. Everything that CAN be tested
> without Unity — the control math, the bridge JSON contract — lives and is tested on the Rust
> side (`rust-core/`, `rust-server/`). Do not treat the C# here as verified until you open it
> in Unity.

---

## The core architectural decision: **Rust is the brain, Unity is the body**

We do **not** re-implement the controller in C#. The whole point of the two-rate brain
(ADR 0001) is one authoritative reflex loop. So:

```
        ┌────────────── Unity (C#) ──────────────┐        ┌──────── Rust (nzi-core) ────────┐
        │  Rigidbody physics + gyro + actuators   │        │  ReflexStabilizer / Attitude     │
        │                                         │        │  Stabilizer (the SAME code that  │
        │  every FixedUpdate:                     │        │  runs on real hardware)          │
        │    1. read angular velocity (gyro)  ────┼──JSON──▶  step(setpoint, measured, dt)    │
        │    2. apply returned torque      ◀──────┼──JSON──   -> per-axis command             │
        └─────────────────────────────────────────┘        └──────────────────────────────────┘
```

Unity sends the **measured body rates**; Rust returns the **actuator commands**. No control
logic lives in C#. This means the sim tests the exact code we ship — no "works in sim, not on
metal" gap.

### Why a socket and not FFI (yet)?
A localhost TCP socket with line-delimited JSON is the simplest thing that decouples the two
runtimes and is trivial to inspect/log. It matches the same "trait seam now, native later"
philosophy as the MeTTa bridge (ADR 0003). A native FFI / shared-memory transport can drop in
behind the same message contract later if the socket round-trip becomes the bottleneck.

---

## The wire contract (authoritative: `BRIDGE_CONTRACT.md`)

One JSON object per line, both directions. See [`BRIDGE_CONTRACT.md`](./BRIDGE_CONTRACT.md) for
the exact fields. The C# structs in `Assets/Nzi/Scripts/BridgeMessages.cs` and the Rust types in
`rust-core/src/unity_bridge.rs` are two views of the SAME contract — change one, change both.

---

## Files

| File | Role |
|---|---|
| `Assets/Nzi/Scripts/NziAgent.cs` | ML-Agents `Agent`: reads gyro, calls the bridge, applies torque. |
| `Assets/Nzi/Scripts/ReflexBridgeClient.cs` | Thin TCP client that talks the JSON contract to Rust. |
| `Assets/Nzi/Scripts/BridgeMessages.cs` | C# structs mirroring the bridge JSON (matches Rust). |
| `BRIDGE_CONTRACT.md` | The language-neutral message contract (source of truth). |
| `SCENE_SETUP.md` | Step-by-step: how to build the scene in the Unity editor. |

## Running it (locally, once you have Unity)

1. Start the Rust bridge server (Phase 1.2 — the socket server that wraps the stabilizer).
2. Open this folder as a Unity project (Unity 2022 LTS+), install the **ML-Agents** package.
3. Follow [`SCENE_SETUP.md`](./SCENE_SETUP.md) to build the one-agent scene.
4. Press Play — the agent should hold attitude, driven by the Rust reflex loop.
