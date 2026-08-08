; Memorization check — catches CONTRADICTING A KNOWN INVARIANT.
; The agent "knows" facts about its current mission mode. A memorization hallucination is when it
; proposes an action that contradicts what it should remember. Concrete Phase-2 invariant:
;   * in mode `hold-level`, the agent MUST keep roll and pitch at zero rate (it may still yaw).
; So a non-zero roll/pitch setpoint while in hold-level is rejected — the agent forgot its mode.
;
; Uses ctx-mode and the sp-* accessors from the other files.

(= (ctx-mode (context $measured $fresh $finite $source $mode)) $mode)

; Is this setpoint consistent with the current mode?
;   free       -> anything within the other checks is fine.
;   hold-level -> roll and pitch rates must be exactly zero.
; Single-`if` membership (no overlapping clauses) so we get exactly one boolean back — see the
; MeTTa non-determinism note in communication.metta.
(= (mode-ok? $mode $action)
   (if (== $mode hold-level)
       (and (== (sp-roll $action) 0)
            (== (sp-pitch $action) 0))
       True))   ; any non-hold-level mode (e.g. `free`) imposes no extra memorized constraint

(= (check-memorization? $action $ctx)
   (mode-ok? (ctx-mode $ctx) $action))
