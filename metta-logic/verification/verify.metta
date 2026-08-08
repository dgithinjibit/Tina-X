; Project Nzi — verification gate composer (P2.1)
; The single entry point the Rust action-gate calls. It runs a proposed action through all five
; hallucination-type checks (defined in the sibling files) and returns:
;   Approved                 -- every check passed
;   (Rejected <check-name>)  -- the FIRST check that failed, naming itself
;
; Order matters only for WHICH reason surfaces first; any failure blocks the action. We check in
; a sensible causal order: can we trust the sensing (perception) and the sender (communication)
; before trusting the plan (reasoning), then whether it's achievable (execution) and consistent
; with what we know (memorization).
;
; This file assumes limits.metta + the five check files are loaded BEFORE it (the Rust gate
; concatenates them in that order; the smoke test loads them explicitly).

(= (gate-setpoint $action $ctx)
   (if (not (check-perception? $action $ctx)) (Rejected perception)
   (if (not (check-communication? $action $ctx)) (Rejected communication)
   (if (not (check-reasoning? $action $ctx)) (Rejected reasoning)
   (if (not (check-execution? $action $ctx)) (Rejected execution)
   (if (not (check-memorization? $action $ctx)) (Rejected memorization)
       Approved))))))
