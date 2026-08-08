# `metta-logic/verification/` — the verification moat (P2.1)

This is Project Nzi's **defensibility**: before the slow brain's proposed action reaches the
reflex/actuator layer, it is run through symbolic checks that catch the five well-known
**agent-hallucination types**. An action that fails any check is *rejected with a reason* — the
agent refuses to act rather than acting confidently-but-wrong.

## The action we verify (Phase 2)

A **setpoint command**: the desired body-rate the brain wants the reflex loop to hold.

```
(setpoint (rates $roll $pitch $yaw))
```

verified against a **context** describing the current situation:

```
(context
  (measured (rates $mr $mp $my))   ; latest gyro reading
  (fresh $bool)                    ; is the telemetry recent enough?
  (finite $bool)                   ; are the sensor values finite (no NaN/inf)?
  (source $id)                     ; who issued this action?
  (mode $mode))                    ; current mission mode, e.g. hold-level | free
```

## The five checks (one file each)

| File | Hallucination type | What it catches for a setpoint |
|---|---|---|
| `perception.metta` | Perception | Acting on stale / non-finite / absent sensor context. |
| `reasoning.metta` | Reasoning | A setpoint that violates physical rate limits (internally impossible). |
| `execution.metta` | Execution | A step change too large for actuator authority (unachievable slew). |
| `memorization.metta` | Memorization | A setpoint that contradicts a known invariant (e.g. `hold-level` mode). |
| `communication.metta` | Communication | An action with a missing/unknown source id (unattributed command). |

`verify.metta` loads all five and exposes the single entry point the Rust gate calls:

```
!(gate-setpoint (setpoint (rates 0.5 0.0 0.2)) <context>)
;; -> Approved            (all checks pass)
;; -> (Rejected perception)   (first failing check names itself)
```

## How it runs

`verify.metta` is the preamble; the Rust action-gate (`nzi-core`, P2.2) appends one
`!(gate-setpoint ...)` line and sends the whole program to MeTTa through the `SubprocessBrain`
seam (ADR 0003). The safety-critical gate decision stays in Rust; only the *reasoning* is
delegated to the MeTTa engine.

Smoke test: `python3 metta-logic/run_verify_smoke.py` (approves a good action, rejects one of
each failure type).
