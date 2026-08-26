; Project TINA-X — symbolic hazard-guidance constraint layer (P5.3).
;
; The ML/ensemble side (weather-prediction/nowcast.py) produces a raw hazard PROBABILITY per cell.
; This symbolic layer turns that number into INTERPRETABLE, better-calibrated guidance by applying
; physical/decision constraints — the WoFS-style "ML over ensemble, made reliable" idea, but with
; the reasoning explicit and auditable (same verifiability principle as the Phase-2 moat and the
; Phase-3 treat gate).
;
; CALIBRATION MODEL — trust the ensemble's own consensus (measured, not assumed).
;   The ensemble SPREAD is the uncertainty signal. When members strongly AGREE (agreement near
;   1.0 = near-unanimous yes, or near 0.0 = near-unanimous no), the extreme probability is
;   trustworthy and should be SHARPENED toward that extreme. When members genuinely DISAGREE
;   (agreement near 0.5), the mid-range probability is honest uncertainty and must be left soft.
;   This is the opposite of blind shrink-to-climatology: we measured that this ensemble is
;   under-confident on true-hazard cells, so shrinking toward the base rate WORSENS reliability
;   (see weather-prediction/tests). Agreement-gated sharpening lowers Brier AND the reliability
;   penalty at every lead time while leaving threshold skill (ETS) intact.
;
; Two jobs:
;   1. CALIBRATE — sharpen p toward its nearer extreme in proportion to ensemble consensus, then
;      add a small bounded persistence nudge (storms don't teleport). One tunable strength.
;   2. CLASSIFY — map the calibrated probability to a named, actionable hazard level with a reason,
;      so a human/agent sees WHY, not just a number.
;
; Inputs:
;   (nowcast (prob $p) (members-agree $frac) (persistence $q))
;     $p    : raw ensemble probability for the cell in [0,1]
;     $frac : fraction of ensemble members agreeing (the consensus/spread proxy) in [0,1]
;     $q    : recent persistence — was this cell hazardous in the previous frame? [0,1]
; Output:
;   (guidance (level <none|watch|warning>) (prob $calibrated) (reason <symbol>))

; Maximum sharpening strength in [0,1]: 0 = trust raw prob, 1 = hard push to the extreme at full
; consensus. 0.5 is the empirically stable choice (monotone reliability gain across all leads).
(= (sharpen-strength) 0.5)
; Decision thresholds on the CALIBRATED probability.
(= (warning-threshold) 0.60)
(= (watch-threshold)   0.30)

; accessors — destructure the full wrapper in one match (MeTTa gotcha).
(= (nc-prob        (nowcast (prob $p) (members-agree $f) (persistence $q))) $p)
(= (nc-agree       (nowcast (prob $p) (members-agree $f) (persistence $q))) $f)
(= (nc-persistence (nowcast (prob $p) (members-agree $f) (persistence $q))) $q)

; |x| — abs is not built-in in this hyperon (MeTTa gotcha); define it locally.
(= (abs $x) (if (< $x 0) (* -1 $x) $x))
; clamp a value into [0,1] so calibration can never emit an out-of-range probability.
(= (clamp01 $x) (if (< $x 0) 0 (if (> $x 1) 1 $x)))

; Ensemble consensus in [0,1]: 0 at maximum spread (agree = 0.5), 1 at unanimity (agree = 0 or 1).
;   consensus = 2 * |agree - 0.5|
(= (consensus $frac) (* 2 (abs (- $frac 0.5))))

; Sharpen p toward its nearer extreme, gated by consensus (so we only sharpen what we trust).
;   calibrated = p + strength * consensus * (p - 0.5)
; High consensus pushes p away from 0.5; low consensus (real disagreement) barely moves it.
(= (calibrate $p $frac)
   (clamp01 (+ $p (* (* (sharpen-strength) (consensus $frac))
                     (- $p 0.5)))))

; A small persistence bonus: a cell hazardous last frame AND still elevated now is more trustworthy.
; Bounded so it can only nudge, not dominate. (Physical consistency: storms don't teleport.)
(= (persistence-bonus $q $p)
   (if (and (> $q 0.5) (> $p (watch-threshold))) 0.05 0.0))

; The guidance rule: calibrate (agreement-gated sharpen), add the bounded persistence nudge, then
; classify with a reason. Single nested `if` (no overlapping clauses -> exactly one guidance atom).
(= (hazard-guidance $nc)
   (let* (($cal (clamp01 (+ (calibrate (nc-prob $nc) (nc-agree $nc))
                            (persistence-bonus (nc-persistence $nc) (nc-prob $nc))))))
     (if (>= $cal (warning-threshold))
         (guidance (level warning) (prob $cal) (reason consensus-sharpened-high))
     (if (>= $cal (watch-threshold))
         (guidance (level watch) (prob $cal) (reason consensus-sharpened-elevated))
         (guidance (level none) (prob $cal) (reason below-watch))))))
