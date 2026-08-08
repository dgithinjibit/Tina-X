; Project Nzi — symbolic treat/don't-treat decision with justification (P3.2).
;
; The ML model (perception/model.py) PERCEIVES: it returns a detected class + confidence. The
; DECISION to spray is made HERE, symbolically, so it is interpretable and verifiable — the same
; principle as the Phase-2 verification moat: an action (spraying pesticide) must be justified, and
; the agent refuses to act when it is not confident.
;
; Input (from the model, see Detection.as_metta_context):
;   (detection (class $c) (confidence $conf))
; Output:
;   (treat <reason>)      -- spray, with the agronomic justification
;   (skip  <reason>)      -- do NOT spray, with the reason (this is the pesticide-saving default)
;
; Design: DON'T-TREAT is the safe default. We spray only when the perception is BOTH a weed AND
; confident enough — an uncertain weed call is treated like "don't know", so we skip rather than
; waste pesticide (and money) on a guess. This makes the pesticide-reduction metric honest: the
; symbolic gate can only ever spray LESS than raw detection, never more.

; Minimum confidence to act on a weed detection. Below this we skip and (in a fuller system) flag
; for human review or a closer pass. Tunable in one place.
(= (treat-confidence-threshold) 0.60)

; Accessors — destructure the full detection wrapper in one match (MeTTa gotcha: partial matches
; fail to reduce).
(= (det-class      (detection (class $c) (confidence $conf))) $c)
(= (det-confidence (detection (class $c) (confidence $conf))) $conf)

; Is the perception confident enough to justify an irreversible spray?
(= (confident? $det) (>= (det-confidence $det) (treat-confidence-threshold)))

; The decision rule. Single nested `if` (no overlapping clauses -> exactly one result).
;   * a confident WEED  -> treat, justified by the detection;
;   * an unconfident weed -> skip (refuse to spray on a low-confidence guess);
;   * crop or soil       -> skip (nothing to treat).
(= (decide-treatment $det)
   (if (== (det-class $det) weed)
       (if (confident? $det)
           (treat confident-weed)
           (skip low-confidence-weed))
       (skip no-weed-detected)))
