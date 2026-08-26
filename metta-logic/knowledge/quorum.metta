; Project TINA-X — honeybee QUORUM + CROSS-INHIBITION collective decision (bee brain upgrade B1–B4).
;
; See docs/research/bee-ant-brain-mechanisms.md. This is the SLOW MeTTa layer for choosing ONE
; shared target among competing options across a SoNS component (e.g. which field/rally-point the
; swarm commits to) — the robotic analog of honeybee nest-site selection (Seeley/Visscher):
;
;   * B1 QUORUM commit  — commit to the best-supported option only once its support crosses a
;     quorum threshold (Seeley proved it's a local COUNT, not global consensus). Below quorum we
;     keep scouting. This is fail-safe: no commitment on weak evidence.
;   * B2 RISK-TUNED quorum — quorum is a speed/accuracy knob (small=fast, large=accurate). We raise
;     it for irreversible/high-risk actions and lower it for time-critical ones (mirrors the single
;     tunable in agronomy/treat.metta).
;   * B3 CROSS-INHIBITION — an option's EFFECTIVE recruitment weight is its raw support minus the
;     stop-signals it received from scouts backing rival options. This breaks deadlock/split between
;     near-equal options that positive feedback alone cannot (Seeley 2012; Laomettachit 2016).
;   * B4 INHIBITION GAIN coupled to quorum — strong cross-inhibition helps at LOW quorum but traps a
;     HIGH-quorum decision in local optima (pitchfork bifurcation, Franci 2015). So gain is ON for
;     low quorum, OFF for high quorum.
;
; The cheap per-agent signals (support tallies, stop-signal emission) live in fast Rust gossip; this
; file is the deliberative arbiter that turns those tallies into a justified commit/scout verdict.
;
; Inputs (built by the Rust supervisor from neighbor-local tallies):
;   context : (qctx (risk <high|low>))
;   option  : (opt $id $support $inhibition)   -- FLAT positional; $inhibition = stop-signals recv'd
;   options : a cons-list of options, (:: <opt> (:: <opt> ()))
; Output:
;   (Commit $id $weight quorum-reached)
;   (Scout below-quorum)                 -- no option has crossed quorum yet (keep gathering)
;
; MeTTa gotcha (learned the hard way, see tina-metta-gotchas): the option is written FLAT as
; (opt $id $s $m) — NOT with (id ..)/(support ..) sub-wrappers. A sub-term like (id north) gets
; EVALUATED (id behaves as identity), collapsing the data before the accessor can match it, leaving
; the whole rule unreduced. Flat positional data has nothing evaluable inside, so accessors fire.

; --- B2: risk-tuned quorum threshold (single place to tune; small=speed, large=accuracy) --------
(= (quorum-threshold (qctx (risk high))) 20)   ; irreversible/high-stakes -> demand strong agreement
(= (quorum-threshold (qctx (risk low)))  6)    ; time-critical/reversible  -> commit sooner

; --- B4: cross-inhibition gain, coupled to the quorum regime -------------------------------------
; ON (1) for low quorum where inhibition breaks ties; OFF (0) for high quorum where it would trap
; the decision in a local optimum. We gate on the risk level directly (it sets the quorum regime).
(= (inhibition-gain (qctx (risk high))) 0)
(= (inhibition-gain (qctx (risk low)))  1)

; --- accessors: destructure the FLAT option in one match (MeTTa gotcha) -------------------------
(= (opt-id         (opt $id $s $m)) $id)
(= (opt-support    (opt $id $s $m)) $s)
(= (opt-inhibition (opt $id $s $m)) $m)

; --- B3: effective recruitment weight = support - gain*inhibition, floored at 0 -----------------
; With gain 0 (high quorum) this is just raw support; with gain 1 (low quorum) rival stop-signals
; subtract from the weight, so a contested option loses ground to a clearly-preferred one.
(= (recruit-weight $ctx $o)
   (let* (($w (- (opt-support $o) (* (inhibition-gain $ctx) (opt-inhibition $o)))))
     (if (< $w 0) 0 $w)))

; --- pick the option with the highest effective weight (best-of-N over a MeTTa list) ------------
; `best-opt` folds a list of options, carrying the current winner. Written as a 2-arg recursion so
; there are no overlapping clauses: an empty tail returns the accumulator; a non-empty tail keeps
; whichever of {acc, head} has the greater recruit-weight.
(= (best-opt $ctx $acc ()) $acc)
(= (best-opt $ctx $acc (:: $head $tail))
   (if (> (recruit-weight $ctx $head) (recruit-weight $ctx $acc))
       (best-opt $ctx $head $tail)
       (best-opt $ctx $acc  $tail)))

; --- B1: the quorum commit decision -------------------------------------------------------------
; Given the context and a NON-EMPTY option list (first + rest), find the best-supported option and
; commit to it ONLY if its effective weight has crossed the risk-tuned quorum; else keep scouting.
; Single nested `if` -> exactly one verdict atom (fail-safe: default is Scout, never a blind commit).
(= (decide-quorum $ctx $first $rest)
   (let* (($best   (best-opt $ctx $first $rest))
          ($weight (recruit-weight $ctx $best)))
     (if (>= $weight (quorum-threshold $ctx))
         (Commit (opt-id $best) $weight quorum-reached)
         (Scout below-quorum))))
