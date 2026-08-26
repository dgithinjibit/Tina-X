; Project TINA-X — partitioned-Atomspace access pattern (P2.7, implements ADR 0002).
;
; THE PROBLEM (measured in P0.3, docs/benchmarks/metta-baseline.md):
;   direct `space.query()` over a flat GroundingSpace is ~O(n) — ~2 s @100k atoms, ~23 s @1M.
;   Rule EVALUATION, by contrast, stayed ~2–7 ms p99 regardless of space size, because a rule
;   that doesn't SCAN the fact base isn't penalized by base size.
;
; THE PATTERN (ADR 0002): partition by concern + recency.
;   * HOT working-space: a handful of facts about the agent's CURRENT state/telemetry. Tiny by
;     construction, so any query over it stays in the fast regime.
;   * COLD knowledge: large/archival facts. We do NOT scan it with query(); we reach specific
;     entries through KEYED rules (equality clauses), which reduce in constant time.
;
; This file demonstrates both halves so the Rust seam + benchmark have a concrete target. The
; hot facts are asserted at runtime by the agent (see AtomspacePartition in Rust); the cold
; lookups are rules keyed by an id — no membership scan.

; --- HOT working-space accessors ------------------------------------------------------------
; The agent asserts its current state as a single wrapper atom so the hot space stays one entry
; per concern. We READ it with an accessor rule (constant cost), never by scanning.
;   (hot-state (rates $r $p $y) (mode $m))
(= (hot-rates (hot-state $rates $mode)) $rates)
(= (hot-mode  (hot-state $rates (mode $m))) $m)

; --- COLD knowledge: keyed rules, NOT scanned -----------------------------------------------
; Example cold table: per-zone wind tolerance. In a flat design you'd store thousands of
; (zone-wind $zone $tol) facts and query() for one — O(n). Instead we express the lookup as a
; keyed rule: asking `(zone-tolerance z3)` reduces directly to its value, independent of how many
; zones exist. Real cold tables are generated/loaded as these keyed clauses.
(= (zone-tolerance default) 12.0)
(= (zone-tolerance calm)    18.0)
(= (zone-tolerance gusty)    6.0)

; A cold lookup with a safe fallback: unknown keys resolve to the default rather than failing or
; scanning. Single-`if` chain (no overlapping clauses) so exactly one value comes back.
(= (lookup-tolerance $zone)
   (if (== $zone calm)  (zone-tolerance calm)
   (if (== $zone gusty) (zone-tolerance gusty)
       (zone-tolerance default))))

; --- bridging hot + cold --------------------------------------------------------------------
; The reasoning the supervisor actually wants: given the agent's hot state and its current zone,
; is the commanded roll rate within that zone's tolerance-derived envelope? This touches ONE hot
; atom and ONE keyed cold rule — no scan of either partition.
(= (within-zone-envelope? $hot $zone)
   (let* (($tol (lookup-tolerance $zone))
          ($rates (hot-rates $hot)))
     (<= (kb-abs (g-roll $rates)) $tol)))
