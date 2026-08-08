; TINA-X — cascading-failure rules (the "domino" logic).
;
; For a junior dev: these rules read the dependency graph (graph.metta) plus live `status` atoms
; and DEDUCE second-order failures. This is the whole point of using symbolic AI: we never trained
; on "quake + typhoon at once"; the rules FOLLOW THE CHAIN and conclude the hospital fails.
;
; Every rule is explainable: a predicted failure is exactly the output of these rules applied to
; the current facts, so an emergency manager can be told WHY.
;
; DESIGN NOTES
;   * `status-of` looks up the current live state of a node.
;   * We treat a road as unusable if flooded OR destroyed (extend the list as needed).
;   * `has-power` is the key derived predicate: a site keeps power if its grid is up OR it has an
;     operable backup generator. A generator is operable only if its fuel-delivery road is usable.
;     That single chain encodes: grid -> backup -> fuel -> road.

; --- helpers ---------------------------------------------------------------------------------

; Current live status of any node (online/offline/flooded/open/...).
(= (status-of $x)
   (match &self (status $x $s) $s))

; Is a road usable for deliveries/access? Unusable if flooded or destroyed.
(= (road-usable $r)
   (if (== (status-of $r) flooded) False
       (if (== (status-of $r) destroyed) False True)))

; Is a power grid currently up?
(= (grid-up $g)
   (if (== (status-of $g) online) True False))

; --- derived failure logic (the cascade) -----------------------------------------------------

; A backup generator can actually RUN only if its fuel-delivery road is usable.
; (Generator exists -> needs fuel -> fuel needs road -> road must be usable.)
(= (generator-operable $gen)
   (match &self (fuel-via $r $gen) (road-usable $r)))

; A site HAS POWER if its grid is up, OR (grid down) it has an operable backup generator.
(= (has-power $site)
   (match &self (powers $g $site)
     (if (grid-up $g)
         True
         (match &self (has-backup $site $gen) (generator-operable $gen)))))

; A site is REACHABLE if at least one of its access roads is usable.
(= (reachable $site)
   (match &self (road-access $r $site) (road-usable $r)))

; A HOSPITAL is CRITICAL if it has lost power (the headline cascade result).
; Returns the hospital id when critical, so callers can collect the list.
(= (hospital-critical $h)
   (if (has-power $h) (empty) $h))

; A hospital is ISOLATED if it still has power but cannot be reached by road.
(= (hospital-isolated $h)
   (if (has-power $h)
       (if (reachable $h) (empty) $h)
       (empty)))

; A DATA CENTER cascade: loses service if its grid is down (no backup modeled) — this is the
; "solar flare -> grid -> data center -> cloud -> hospital software" chain seed.
(= (datacenter-down $d)
   (match &self (powers $g $d)
     (if (grid-up $g) (empty) $d)))
