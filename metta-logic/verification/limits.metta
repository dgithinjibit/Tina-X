; Project TINA-X — shared physical limits & helpers for the verification checks (P2.1)
; These are the tunable constants the five checks reference. Keeping them in one place means
; a junior dev changes a limit HERE, not scattered across five files.

; --- physical envelope (body rates, rad/s) ---
(= (max-rate) 3.0)          ; largest attitude rate we ever command on any axis
(= (max-rate-step) 1.5)     ; largest change-from-current we allow in one setpoint (slew/authority)

; --- helpers ------------------------------------------------------------------------------
; absolute value (MeTTa has no built-in abs we rely on; define it explicitly for clarity)
(= (abs $x) (if (< $x 0) (* -1 $x) $x))

; all-three: True only if all three axis values satisfy predicate P (passed as a symbol we
; compare against True). We spell it out per-axis rather than folding, to stay readable.
(= (within-mag? $v $limit) (<= (abs $v) $limit))
