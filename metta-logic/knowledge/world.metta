; Project TINA-X — WORLD facts (P2.4, supervisory-loop knowledge base).
; What the agent knows about its ENVIRONMENT and its own physical envelope. These are the
; ground truths the slow brain reasons over when proposing a setpoint. They are deliberately the
; SAME constants the verification checks enforce (limits.metta) so a proposal the brain makes is
; consistent with what the gate will allow — the brain shouldn't propose what the gate must reject.
;
; MeTTa gotchas we respect (see memory / verification/communication.metta):
;   * no overlapping equality clauses (they cause non-deterministic [A,B] results);
;   * accessors must destructure the FULL wrapper in one match.

; --- physical envelope (body rates, rad/s) — mirror of verification/limits.metta ------------
(= (world-max-rate) 3.0)        ; largest attitude rate we will ever command on any axis
(= (world-max-rate-step) 1.5)   ; largest change-from-current allowed in one setpoint (slew)

; --- environment ----------------------------------------------------------------------------
; A coarse wind tolerance (m/s) reused from the Phase-0 safe-to-fly rule. Above this the world is
; too hostile to command aggressive rates; the brain trims its proposal (see mission.metta).
(= (world-wind-tolerance) 12.0)

; abs helper, defined locally so the KB is self-contained (MeTTa has no built-in abs we rely on).
(= (kb-abs $x) (if (< $x 0) (* -1 $x) $x))

; Clamp a value into [-limit, limit]. Used to keep a proposed rate inside the envelope BEFORE it
; ever reaches the gate, so a well-behaved brain proposal is approved by construction.
(= (clamp-mag $x $limit)
   (if (> $x $limit) $limit
   (if (< $x (* -1 $limit)) (* -1 $limit)
       $x)))
