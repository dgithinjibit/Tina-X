# Scene setup — one Nzi agent (Phase 1.1)

Step-by-step to build the minimal scene in the Unity editor. Assumes Unity **2022 LTS** or newer
and the **ML-Agents** package installed (Window ▸ Package Manager ▸ add `com.unity.ml-agents`).

> Reminder: the control loop lives in **Rust**. Start the Rust reflex bridge server first
> (Phase 1.2), then press Play. Without it, the agent free-falls (by design — crash tolerant).

## 1. The agent body
1. Create an empty GameObject, name it `NziAgent`.
2. Add a child mesh (a `Cube` or a simple quad-rotor model) so you can see its attitude.
3. On `NziAgent` add:
   - **Rigidbody** — uncheck *Use Gravity* for a pure attitude test first; enable it later.
   - **NziAgent.cs** (this scaffold) — set `Host`/`Port` to match the Rust server (`127.0.0.1:8765`).
   - **Behavior Parameters** (from ML-Agents) — leave the model empty for Phase 1; we drive via
     the bridge, not a trained policy.

## 2. The world
1. Add a `Plane` as the ground.
2. Scatter a few `Cube`/`Cylinder` obstacles (Phase 1.3 will add optic-flow avoidance against them).
3. Add a light and a camera framing the agent.

## 3. Fixed timestep
- Edit ▸ Project Settings ▸ Time ▸ **Fixed Timestep** = `0.002` (500 Hz) to match
  `REFLEX_TARGET_HZ` in `rust-core`. The bridge sends `dt` each tick, so Rust adapts if you
  change it, but 500 Hz keeps sim and hardware assumptions aligned.

## 4. Axis mapping (IMPORTANT)
`NziAgent.cs` assumes **roll = z, pitch = x, yaw = y** in Unity's left-handed frame. If your
model's forward/up axes differ, fix the two mapping lines in `FixedUpdate` (marked with the
convention comment) — get this wrong and the agent will fight itself. Verify by nudging one axis
and confirming only that axis's command responds.

## 5. Run
1. Start the Rust bridge server (Phase 1.2).
2. Press Play. Drop the agent with a random initial rotation (`OnEpisodeBegin` does this) and
   watch the reflex loop bring it level.
3. Watch the Console for `reflex step over budget` warnings — there should be none.

## What "done" looks like (P1.1–P1.2 exit)
The agent holds attitude from a random disturbed start, driven entirely by the Rust reflex loop,
at 500 Hz, with reflex latency well under 13 ms. Then move on to optic-flow avoidance (P1.3).
