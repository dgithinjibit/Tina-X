# Project TINA-X — MeTTa/Hyperon hackathon grill

## Executive assessment

**Recommendation: submit, but narrow the live pitch.** TINA-X has a credible MeTTa/Hyperon core and more implementation evidence than a typical concept-only submission. It is not yet a credible “complete autonomous swarm” product. Its strongest competitive position is a reusable symbolic supervisory layer that turns facts and rules into an explainable, verified action across multiple domains.

The official Hyperon overview emphasizes Atomspace metagraphs, MeTTa’s multiparadigm reasoning model, interaction with external processes, and cognitive-system composition [1]. TINA-X’s bridge, rule files, Atomspace partitioning decision, TINA-X cascade engine, and downstream Rust gate map to those strengths. The ecosystem itself is described as active pre-alpha software [2], so transparent limitations are a strength rather than an embarrassment.

## Scoring rubric

| Criterion | Current assessment | What must be shown |
|---|---|---|
| MeTTa depth | **Strong foundation, weak first impression** | Live rule execution, an explanation trace, and the exact Atomspace facts involved. |
| Novelty | **Strong** | The two-rate “brain proposes, gate verifies, reflex executes” pattern and its reuse in TINA-X/agriculture/swarm. |
| Working implementation | **Good prototype** | One deterministic end-to-end command that changes an action after a MeTTa result. |
| Technical clarity | **Mixed** | Reduce equal-weight narratives; identify one primary demo and classify all other tracks. |
| Real-world relevance | **Strong but broad** | Choose one beneficiary and quantify one decision-level outcome, not a collection of future benefits. |
| Evidence | **Good for a prototype** | Keep local benchmark methodology, environment versions, and limitations visible. |

## Hard questions judges may ask

### “Why is this not just a Rust application with a MeTTa label?”

Answer by opening the rule file and running it. Show a fact entering the Atomspace, a rule deriving a decision, the result crossing the `SymbolicBrain` seam, and the Rust gate accepting or rejecting it. The MeTTa artifact must be load-bearing: changing the rule or fact must change the outcome.

### “Why not use a neural classifier or a normal rules engine?”

Do not claim that deep learning “fails” in general. Demonstrate the narrower advantage: TINA-X composes explicit dependencies to derive a compound cascade from a scenario combination, while the agriculture gate makes constraints and exceptions inspectable. The value is compositionality and traceability under changing facts, not magic generalization.

### “Why so many domains?”

Explain that agriculture, weather, swarms, and TINA-X are testbeds for one substrate. During the live pitch, show only two. Label weather, GNSS/EO, ZK, Unity, and Robonomics as research extensions or integration tracks unless they are part of the submitted demo.

### “Is it real-time?”

Say precisely: the reflex loop is the real-time path; MeTTa is supervisory and intentionally excluded from the reflex path. TINA-X’s local benchmarks measured rule evaluation at millisecond scale and observed flat-space query growth, which motivated partitioning. These are project measurements, not universal Hyperon guarantees.

### “What is actually finished?”

Finished enough for a software prototype: MeTTa smoke tests and bridge worker, Rust trait seam and supervisor logic, headless swarm logic, TINA-X standalone reasoner, weather baseline/guidance experiments, dashboard plumbing, and a Rust ZK POC. Not finished: field robotics, embedded Hyperon deployment, Unity end-to-end scene, production weather validation, Robonomics deployment, and large-scale infrastructure data.

## Five-minute winning demo

1. Run `metta-logic/run_smoke.py` and show a rule result.
2. Run the Rust bridge worker or `brain-demo` and show MeTTa output crossing into Rust.
3. Trigger one agriculture or safety-gate case where the symbolic result changes the action.
4. Run TINA-X on a compound earthquake/typhoon scenario and show the derived cascade and explanation.
5. Change one fact or rule and rerun, proving that the result is reasoned rather than hard-coded.

The dashboard should be used only if it makes the decision trace clearer. A polished UI cannot compensate for an unclear symbolic mechanism.

## Required submission discipline

Use the README’s “What TINA-X is—and is not” table to prevent overclaiming. Put all performance numbers beside their benchmark command and environment. Keep a short architecture diagram in the submission. Add one screenshot or terminal transcript showing the MeTTa result, the explanation, and the downstream action. If the event supplies a formal rubric, map this document’s criteria to it instead of inventing stronger claims.

## References

[1]: https://hyperon.opencog.org/ "OpenCog Hyperon — Atomspace, MeTTa, and cognitive architecture"
[2]: https://github.com/trueagi-io/hyperon-experimental "trueagi-io/hyperon-experimental — MeTTa implementation"
[3]: https://singularitynet.io/research/metta-programming-language/ "SingularityNET — MeTTa Programming Language"
[4]: https://www.blockchaincentrenbo.com/events/metta-training-hackathon-2025/ "MeTTa Training/Hackathon 2025 — developer tracks"
