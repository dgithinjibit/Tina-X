// The one TINA-X agent for Phase 1: a rigid body whose attitude is stabilized by the RUST reflex
// loop over the bridge. This is the "body"; Rust is the "brain" (ADR 0001).
//
// Every fixed physics tick we:
//   1. read the body's angular velocity (our simulated gyro / haltere analogue),
//   2. send it to Rust and get back a per-axis actuator command,
//   3. apply that command as torque.
//
// There is deliberately NO PID / control math here — that would duplicate (and drift from) the
// authoritative Rust controller. Keeping it in one place is the whole point of the design.
//
// ML-Agents note: we inherit from `Agent` so this slots into an ML-Agents scene, but Phase 1
// uses the bridge for control (not a trained policy). Later phases can add observations/rewards
// on top; the reflex loop stays underneath as the stable "spine".
//
// NOTE: scaffold — not compiled in this repo's dev container. See SCENE_SETUP.md to wire it up.

using Unity.MLAgents;
using UnityEngine;

namespace TINA-X
{
    [RequireComponent(typeof(Rigidbody))]
    public sealed class TinaXAgent : Agent
    {
        [Tooltip("Scales the [-1,1] command from Rust into physical torque (N·m).")]
        public float torqueScale = 0.5f;

        [Tooltip("Rust reflex bridge host/port (see BRIDGE_CONTRACT.md).")]
        public string host = "127.0.0.1";
        public int port = 8765;

        private Rigidbody _body;
        private ReflexBridgeClient _bridge;
        private ulong _step;

        public override void Initialize()
        {
            _body = GetComponent<Rigidbody>();
            _bridge = new ReflexBridgeClient(host, port);
        }

        // Physics runs in FixedUpdate — the correct place for a fixed-rate control loop.
        private void FixedUpdate()
        {
            // 1. Read the gyro: Unity gives angular velocity in world space; for a first cut we
            //    treat it as body rates (roll=x, pitch=y? ...). Map axes carefully to your rig in
            //    SCENE_SETUP.md; the mapping below is the convention this scaffold assumes.
            Vector3 w = transform.InverseTransformDirection(_body.angularVelocity);
            var reading = new GyroReading
            {
                step = _step,
                dt = Time.fixedDeltaTime,
                measured = new Axis3(w.z, w.x, w.y), // roll=z, pitch=x, yaw=y (Unity convention)
            };

            // 2. Ask Rust for the command. If the exchange fails, apply zero torque this tick
            //    (crash-tolerant: the agent free-falls briefly rather than the sim hanging).
            Vector3 torque = Vector3.zero;
            if (_bridge.Exchange(reading, out ActuatorCommand cmd))
            {
                // Map the per-axis command back into a Unity torque vector (inverse of above).
                torque = new Vector3(cmd.command.pitch, cmd.command.yaw, cmd.command.roll) * torqueScale;

                // Surface the reflex latency in the editor so you can watch the 13 ms budget.
                if (cmd.latency_us >= 13000)
                    Debug.LogWarning($"[TINA-X] reflex step over budget: {cmd.latency_us} µs");
            }

            // 3. Apply it in body frame.
            _body.AddRelativeTorque(torque, ForceMode.Acceleration);
            _step++;
        }

        // Reset the body when an episode begins (ML-Agents lifecycle hook).
        public override void OnEpisodeBegin()
        {
            _body.angularVelocity = Vector3.zero;
            _body.velocity = Vector3.zero;
            transform.localRotation = Random.rotation; // start disturbed; the reflex loop recovers
            _step = 0;
        }

        private void OnDestroy()
        {
            _bridge?.Dispose();
        }
    }
}
