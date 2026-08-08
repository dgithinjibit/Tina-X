; Perception check — catches acting on BAD SENSING.
; A setpoint decision is only trustworthy if the context it was based on is trustworthy:
;   * the telemetry must be FRESH (not stale), and
;   * the sensor values must be FINITE (no NaN/inf from a broken gyro).
; If perception is compromised, the safest action is to NOT act on it.
;
; Context shape (see README):
;   (context (measured ...) (fresh $b) (finite $b) (source $id) (mode $m))

; Extract the two perception flags from a context.
(= (ctx-fresh  (context $measured $fresh $finite $source $mode)) $fresh)
(= (ctx-finite (context $measured $fresh $finite $source $mode)) $finite)

; PASS only if the telemetry is both fresh AND finite.
(= (check-perception? $action $ctx)
   (and (== (ctx-fresh $ctx) True)
        (== (ctx-finite $ctx) True)))
