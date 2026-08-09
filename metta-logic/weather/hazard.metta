; Project Nzi — symbolic hazard-guidance constraint layer (P5.3).
;
; The ML/ensemble side (weather-prediction/nowcast.py) produces a raw hazard PROBABILITY per cell.
; This symbolic layer turns that number into INTERPRETABLE, better-calibrated guidance by applying
; physical/decision constraints — the WoFS-style "ML over ensemble, made reliable" idea, but with
; the reasoning explicit and auditable (same verifiability principle as the Phase-2 moat and the
; Phase-3 treat gate).
;
; Two jobs:
;   1. CALIBRATE — raw ensemble probabilities are typically OVERCONFIDENT at the extremes (a cell
;      where 100% of members agree is rarely truly 100%). We shrink probabilities gently toward the
;      climatological base rate, which improves reliability (this is exactly what the Brier /
;      reliability metrics reward). One tunable strength in one place.
;   2. CLASSIFY — map a calibrated probability + local persistence context to a named, actionable
;      hazard level with a reason, so a human/agent sees WHY, not just a number.
;
; Inputs:
;   (nowcast (prob $p) (members-agree $frac) (persistence $q))
;     $p    : raw ensemble probability for the cell in [0,1]
;     $frac : fraction of ensemble members agreeing (a confidence/spread proxy)
;     $q    : recent persistence — was this cell hazardous in the previous frame? [0,1]
; Output:
;   (guidance (level <none|watch|warning>) (prob $calibrated) (reason <symbol>))

; Climatological base rate for severe convection in a cell (rare!). Calibration shrinks toward it.
(= (climo-base-rate) 0.08)
; Shrink strength in [0,1]: 0 = trust raw prob, 1 = ignore it. Modest so we keep signal.
(= (calib-strength) 0.25)
; Decision thresholds on the CALIBRATED probability.
(= (warning-threshold) 0.60)
(= (watch-threshold)   0.30)

; accessors — destructure the full wrapper in one match (MeTTa gotcha).
(= (nc-prob        (nowcast (prob $p) (members-agree $f) (persistence $q))) $p)
(= (nc-agree       (nowcast (prob $p) (members-agree $f) (persistence $q))) $f)
(= (nc-persistence (nowcast (prob $p) (members-agree $f) (persistence $q))) $q)

; Shrink a probability toward the climatological base rate (reliability calibration).
;   calibrated = (1-s)*p + s*base
(= (calibrate $p)
   (+ (* (- 1 (calib-strength)) $p)
      (* (calib-strength) (climo-base-rate))))

; A small persistence bonus: a cell hazardous last frame AND flagged now is more trustworthy.
; Bounded so it can only nudge, not dominate. (Physical consistency: storms don't teleport.)
(= (persistence-bonus $q $p)
   (if (and (> $q 0.5) (> $p (watch-threshold))) 0.05 0.0))

; The guidance rule: calibrate, add the bounded persistence nudge, then classify with a reason.
; Single nested `if` (no overlapping clauses -> exactly one guidance atom).
(= (hazard-guidance $nc)
   (let* (($cal (+ (calibrate (nc-prob $nc))
                   (persistence-bonus (nc-persistence $nc) (nc-prob $nc)))))
     (if (>= $cal (warning-threshold))
         (guidance (level warning) (prob $cal) (reason calibrated-high-confidence))
     (if (>= $cal (watch-threshold))
         (guidance (level watch) (prob $cal) (reason calibrated-elevated))
         (guidance (level none) (prob $cal) (reason below-watch))))))
