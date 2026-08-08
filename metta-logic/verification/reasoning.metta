; Reasoning check — catches an INTERNALLY IMPOSSIBLE setpoint.
; The brain might "reason" its way to a command that violates the physical envelope (e.g. asking
; for a 50 rad/s spin). That's a reasoning hallucination: the conclusion is unphysical regardless
; of the inputs. We reject any setpoint whose magnitude on ANY axis exceeds (max-rate).
;
; Setpoint shape: (setpoint (rates $roll $pitch $yaw))

; Pull the three requested rates out of a setpoint action.
(= (sp-roll  (setpoint (rates $r $p $y))) $r)
(= (sp-pitch (setpoint (rates $r $p $y))) $p)
(= (sp-yaw   (setpoint (rates $r $p $y))) $y)

; PASS only if every axis is within the max-rate envelope.
(= (check-reasoning? $action $ctx)
   (and (within-mag? (sp-roll  $action) (max-rate))
   (and (within-mag? (sp-pitch $action) (max-rate))
        (within-mag? (sp-yaw   $action) (max-rate)))))
