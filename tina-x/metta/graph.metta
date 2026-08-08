; TINA-X — the infrastructure dependency graph (the "skeleton").
;
; For a junior dev: this is the KNOWLEDGE GRAPH. Each line is an atom (a fact) describing either
; a piece of infrastructure, a dependency between two pieces, or a live status. TINA-X reasons
; OVER this graph to predict cascades. In production these atoms would be scraped from
; OpenStreetMap (Phase 1); here we hand-build a small Kyushu-like region so the logic is testable.
;
; NAMING CONVENTION (keep it consistent — the rules match on these shapes):
;   (: <id> <Type>)            -> typing: what kind of thing this is
;   (powers <grid> <site>)     -> <grid> supplies electricity to <site>
;   (has-backup <site> <gen>)  -> <site> has backup generator <gen>
;   (fuel-via <road> <gen>)    -> <gen>'s diesel is delivered over <road>
;   (road-access <road> <site>)-> <site> is reachable via <road> (ambulances, staff)
;   (status <id> <state>)      -> live/dynamic state (online/offline/flooded/damaged/ok)

; ---- Types (what each node IS) --------------------------------------------------------------
(: grid-A PowerGrid)
(: grid-B PowerGrid)
(: hospital-B Hospital)
(: hospital-C Hospital)
(: substation-X Substation)
(: generator-1 Generator)
(: generator-2 Generator)
(: road-3 Road)
(: road-5 Road)
(: datacenter-1 DataCenter)

; ---- Dependency edges (who depends on whom) -------------------------------------------------
; Power distribution
(feeds substation-X grid-A)          ; substation feeds the grid
(powers grid-A hospital-B)           ; grid-A powers hospital-B
(powers grid-A datacenter-1)         ; ...and a data center
(powers grid-B hospital-C)           ; grid-B powers hospital-C

; Backup generators + their fuel logistics
(has-backup hospital-B generator-1)
(has-backup hospital-C generator-2)
(fuel-via road-3 generator-1)        ; generator-1 refuels over road-3
(fuel-via road-5 generator-2)        ; generator-2 refuels over road-5

; Physical access (ambulances, staff, supplies)
(road-access road-3 hospital-B)
(road-access road-5 hospital-C)

; ---- Baseline live status (everything nominal until an event changes it) --------------------
(status grid-A online)
(status grid-B online)
(status substation-X online)
(status road-3 open)
(status road-5 open)
(status hospital-B operational)
(status hospital-C operational)
(status datacenter-1 online)
