; Project Nzi — AGENT self-facts (P2.4).
; What the agent knows about ITSELF: identity and provenance. The `agent-id` here is the source
; the supervisor stamps onto proposed actions, and it must be on the verification allow-list
; (verification/communication.metta trusts `local-brain`) — otherwise the agent's OWN proposals
; would be rejected as unattributed. Keeping this fact here makes the linkage explicit.

; The identity the slow brain issues commands under. MUST match a trusted source in
; verification/communication.metta (currently `local-brain`).
(= (agent-id) local-brain)

; A coarse self-confidence the governance layer can consult. Phase 2 keeps this a constant;
; later phases can make it reflect sensor health / knowledge-boundary distance. Governance treats
; a low confidence like a refusal (hold last-safe) — see docs/adr/0006-supervisory-loop.md.
(= (agent-confidence) high)
(= (confidence-ok? $c) (== $c high))
