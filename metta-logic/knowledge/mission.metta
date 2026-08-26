; Project TINA-X — MISSION facts + the setpoint decision rule (P2.4).
; The mission GOAL is expressed as a desired attitude-rate target per mode. `decide-setpoint`
; is the slow brain's reasoning entry point: given the current mission mode and the latest
; telemetry, it PROPOSES a setpoint. The Rust supervisor then runs that proposal through the
; verification gate (Stage ①) before any of it reaches the reflex loop.
;
; Shapes (match the Rust supervisor + verification README):
;   telemetry: (telemetry (measured (rates $r $p $y)) (mode $mode))
;   proposal : (setpoint (rates $r $p $y))      -- same term the gate verifies
;
; NOTE: this file assumes world.metta is loaded first (it uses clamp-mag / world-max-rate).

; --- mission goal per mode ------------------------------------------------------------------
; The target body-rate the mission wants, by mode. Single-`if` dispatch (no overlapping clauses)
; so exactly one target comes back.
;   hold-level -> hold attitude: zero roll/pitch rate, zero yaw (the memorization invariant).
;   free       -> a gentle default forward-authority target; real missions override this.
(= (goal-rates $mode)
   (if (== $mode hold-level) (rates 0 0 0)
       (rates 0.5 0.0 0.2)))   ; `free` and anything else: a modest, in-envelope target

; --- telemetry accessors (destructure the full wrapper in one match) -------------------------
(= (tlm-mode     (telemetry $measured $mode-wrap)) (mode-val $mode-wrap))
(= (mode-val     (mode $m)) $m)
(= (tlm-measured (telemetry $measured $mode-wrap)) $measured)
(= (meas-roll    (measured (rates $r $p $y))) $r)
(= (meas-pitch   (measured (rates $r $p $y))) $p)
(= (meas-yaw     (measured (rates $r $p $y))) $y)

; goal-rates accessors so we can clamp each axis of the chosen target.
(= (g-roll  (rates $r $p $y)) $r)
(= (g-pitch (rates $r $p $y)) $p)
(= (g-yaw   (rates $r $p $y)) $y)

; --- the decision rule ----------------------------------------------------------------------
; Propose a setpoint for the given telemetry:
;   1. pick the mission target for the current mode,
;   2. clamp each axis into the physical envelope (so the proposal is gate-consistent by
;      construction — the brain never knowingly proposes an over-envelope rate).
; The Rust supervisor takes this proposal and STILL runs it through the full gate; clamping here
; just means a healthy proposal sails through rather than being rejected on `reasoning`.
(= (decide-setpoint $telemetry)
   (let* (($goal (goal-rates (tlm-mode $telemetry))))
     (setpoint (rates
       (clamp-mag (g-roll  $goal) (world-max-rate))
       (clamp-mag (g-pitch $goal) (world-max-rate))
       (clamp-mag (g-yaw   $goal) (world-max-rate))))))
