; Communication check — catches an UNATTRIBUTED / MALFORMED command.
; Every action must come from a known, valid source (the local brain, or a trusted peer in the
; swarm). A communication hallucination is acting on a command whose provenance is missing or
; unrecognized — e.g. a corrupted swarm message or a spoofed setpoint. We only trust sources on
; an allow-list.
;
; Uses ctx-source (defined here) and an explicit list of trusted sources.

(= (ctx-source (context $measured $fresh $finite $source $mode)) $source)

; The allow-list of trusted issuers. `unknown` is the sentinel the Rust gate uses when a command
; arrived with no valid attribution — it is deliberately NOT trusted.
;
; NOTE (MeTTa gotcha): we do NOT write overlapping equality clauses like
;   (= (trusted? local-brain) True)  (= (trusted? $x) False)
; because BOTH would fire for `local-brain` and MeTTa would return [True, False] (non-determinism).
; Instead we test membership with a single `if`, which yields exactly one boolean.
(= (trusted-source? $s)
   (if (== $s local-brain) True
   (if (== $s swarm-peer)  True
       False)))

(= (check-communication? $action $ctx)
   (trusted-source? (ctx-source $ctx)))
