// C# view of the Unity <-> Rust reflex bridge contract.
//
// These structs mirror unity-sim/BRIDGE_CONTRACT.md and rust-core/src/unity_bridge.rs EXACTLY.
// If you change a field here, change it in the Rust types and the contract doc too, or the two
// runtimes will silently disagree.
//
// NOTE: scaffold — not compiled in this repo's dev container. Drop into a Unity project to use.
// We use Unity's built-in JsonUtility, so field names must match the JSON keys (roll/pitch/yaw...)
// and types must be plain serializable fields (no properties).

using System;

namespace TINA-X
{
    /// <summary>A roll/pitch/yaw triple of body-frame angular rates or commands.</summary>
    [Serializable]
    public struct Axis3
    {
        public float roll;
        public float pitch;
        public float yaw;

        public Axis3(float roll, float pitch, float yaw)
        {
            this.roll = roll;
            this.pitch = pitch;
            this.yaw = yaw;
        }
    }

    /// <summary>Unity -> Rust: one gyro reading for a fixed physics tick.</summary>
    [Serializable]
    public struct GyroReading
    {
        public ulong step;      // FixedUpdate counter
        public float dt;        // Time.fixedDeltaTime
        public Axis3 measured;  // body angular rates (rad/s) from the gyro
    }

    /// <summary>Rust -> Unity: the actuator command to apply this tick.</summary>
    [Serializable]
    public struct ActuatorCommand
    {
        public ulong step;       // echoes the request step
        public Axis3 command;    // per-axis actuator command, nominally [-1, 1]
        public uint latency_us;  // Rust reflex-step cost; must stay < 13000
    }
}
