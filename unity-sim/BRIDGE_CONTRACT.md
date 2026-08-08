# Unity ↔ Rust reflex bridge — message contract

**Source of truth** for the wire format between the Unity sim (C#) and the Rust reflex loop.
Both `unity-sim/Assets/Nzi/Scripts/BridgeMessages.cs` and `rust-core/src/unity_bridge.rs` must
match this document. If you change a field here, change it in BOTH and update their tests.

## Transport
- **Localhost TCP**, default `127.0.0.1:8765`.
- **Line-delimited JSON**: exactly one JSON object per line, terminated by `\n`.
- **Request/response, lock-step**: Unity sends one `GyroReading`, Rust replies with one
  `ActuatorCommand`. One exchange per Unity `FixedUpdate` (the fixed physics tick).
- All rates are **body-frame angular velocity in rad/s**; commands are **torque-ish scalars**
  in `[-1, 1]` per axis (the C# side scales them to physical torque).

## Unity → Rust: `GyroReading`
```json
{ "step": 1234, "dt": 0.002, "measured": { "roll": 0.12, "pitch": -0.03, "yaw": 0.00 } }
```
| field | type | meaning |
|---|---|---|
| `step` | u64 | Unity's FixedUpdate counter (for correlating logs). |
| `dt` | f32 | Fixed timestep in seconds (Unity `Time.fixedDeltaTime`). |
| `measured` | {roll,pitch,yaw} f32 | Body angular rates from the simulated gyro. |

## Rust → Unity: `ActuatorCommand`
```json
{ "step": 1234, "command": { "roll": 0.4, "pitch": -0.1, "yaw": 0.0 }, "latency_us": 7 }
```
| field | type | meaning |
|---|---|---|
| `step` | u64 | Echoes the request `step` so Unity can assert alignment. |
| `command` | {roll,pitch,yaw} f32 | Per-axis actuator command from the stabilizer. |
| `latency_us` | u32 | How long the Rust reflex step took (µs) — must stay < 13 000. |

## Setpoints
The **setpoint** (desired body rate) is owned by Rust — it comes from the slow MeTTa brain, not
from Unity. For Phase 1 the setpoint defaults to "hold zero rotation" (`{0,0,0}`), i.e. keep the
body level. A later `Setpoint` message (Rust-internal, not on this Unity wire) lets the brain
change it. Unity never sets the setpoint; it only reports what it measures and applies what it's
told. This keeps the "Rust is the brain" boundary clean.

## Error handling
- If Rust doesn't reply within one fixed timestep, Unity applies **zero torque** for that tick
  and logs a dropped frame (never blocks the physics thread indefinitely).
- If the socket drops, Unity retries the connection with backoff; the agent free-falls until
  reconnected (crash-tolerance is a feature — see the fly's collision tolerance).
