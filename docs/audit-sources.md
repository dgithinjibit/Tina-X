# Audit sources

## Hyperon / MeTTa primary source

- [OpenCog Hyperon experimental repository](https://github.com/trueagi-io/hyperon-experimental): the project describes OpenCog Hyperon as an active pre-alpha stage of development and experimentation, with MeTTa (Atomese 2) intended to support clear semantics, meta-language features, and different inference types. The repository exposes a Python package version 0.2.10 in its current public page and provides examples and documentation links.

## Local audit evidence

- `metta-logic/` contains MeTTa rules, smoke tests, benchmarks, a bridge worker, quorum logic, and weather hazard guidance.
- `rust-core/` contains a delayed-PD reflex loop, a typed `SymbolicBrain` trait, `SubprocessBrain`, supervisory gate, quorum arbiter, telemetry, and roofline reasoning.
- `rust-swarm/` contains a headless self-organizing nervous system simulation with hierarchy/election, stigmergy, coverage, and hazard overlay logic.
- `tina-x/` contains an independently runnable MeTTa cascading-failure reasoner and scenarios.
- `weather-prediction/` contains a synthetic calibrated nowcasting baseline, MeTTa guidance, thermodynamic/p-bit denoising, and sensor/EO/GNSS bridge experiments, but explicitly marks real-world reliability and the unified stack as unproven.
- Precision-agriculture implementation exists in `perception/` with associated MeTTa agronomy rules in `metta-logic/agronomy/treat.metta`, but the README currently foregrounds the project as a broad bio-inspired swarm platform rather than making the concrete MeTTa decision-verification demo the primary hackathon wedge.
- Unity and Robonomics are scaffold/deferred areas, not completed end-to-end integrations.

## Initial documentation risks

1. The README’s MeTTa/Hyperon presence is technically mentioned, but the concrete symbolic artifacts and measurable reasoning results are buried below the top-level vision.
2. The README leads with fly biomimicry and precision agriculture, which can make the submission appear robotics-heavy rather than a MeTTa/Hyperon hackathon project.
3. TINA-X and weather nowcasting are distinct applications of the same symbolic layer; the relationship and boundaries need to be explained explicitly.
4. Claimed test counts and quick-start commands should be revalidated against the current workspace and environment; Rust tooling is not installed in this sandbox, so local verification is pending.
5. The README needs a clear judge-facing demo path: run MeTTa directly, show a rule firing/explanation, then show Rust consuming the result and the swarm/agriculture gate acting on it.
