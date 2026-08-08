; Execution check — catches an UNACHIEVABLE step change.
; A setpoint can be individually within limits yet demand a jump from the CURRENT measured rate
; that the actuators can't deliver in one control step. That's an execution hallucination: the
; plan ignores what the body can actually do. We reject if the change on any axis exceeds
; (max-rate-step).
;
; Uses sp-* (from reasoning.metta) for the requested rates and the measured rates from context.

; Pull the three measured rates out of a context's (measured (rates ...)) block. Note the
; accessors unwrap the `measured` LABEL and the `rates` constructor in one match, so we read the
; raw numbers directly (avoids an intermediate that fails to reduce).
(= (ctx-measured (context $measured $fresh $finite $source $mode)) $measured)
(= (m-roll  (measured (rates $r $p $y))) $r)
(= (m-pitch (measured (rates $r $p $y))) $p)
(= (m-yaw   (measured (rates $r $p $y))) $y)

; Per-axis step magnitude = |requested - measured|.
(= (step-roll  $action $ctx) (abs (- (sp-roll  $action) (m-roll  (ctx-measured $ctx)))))
(= (step-pitch $action $ctx) (abs (- (sp-pitch $action) (m-pitch (ctx-measured $ctx)))))
(= (step-yaw   $action $ctx) (abs (- (sp-yaw   $action) (m-yaw   (ctx-measured $ctx)))))

; PASS only if every axis's step is within actuator authority.
(= (check-execution? $action $ctx)
   (and (<= (step-roll  $action $ctx) (max-rate-step))
   (and (<= (step-pitch $action $ctx) (max-rate-step))
        (<= (step-yaw   $action $ctx) (max-rate-step)))))
