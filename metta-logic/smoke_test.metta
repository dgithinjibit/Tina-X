; Project TINA-X — MeTTa smoke test (P0.2)
; Minimal program: assert facts into the Atomspace and run queries.
; Run via the Python harness: python3 metta-logic/run_smoke.py
; This proves MeTTa is installed and reasoning works before we build the symbolic brain.

; --- facts: a tiny "world model" for one agent ---
(= (agent-id) tina-001)
(= (max-wind-tolerance-ms) 12)      ; agent can operate up to 12 m/s wind

; --- a rule: is it safe to fly given a wind reading? ---
; (safe-to-fly? W) -> True if W <= tolerance, else False
(= (safe-to-fly? $w)
   (if (<= $w (max-wind-tolerance-ms)) True False))

; --- queries (each ! line is evaluated) ---
!(agent-id)                 ; expect: tina-001
!(safe-to-fly? 8)           ; expect: True
!(safe-to-fly? 20)          ; expect: False
