# Weather Prediction Edge Case — Harsh-Weather Nowcasting

> Flagship edge case for Project TINA-X: real-time detection/prediction of harsh weather
> (hailstorms, severe convection) so a swarm can *predict-and-act* — e.g. self-shelter,
> reroute, or warn a community — before the event.
> All figures below are cited and adversarially verified (deep-research run, 2026-08-08).

## Honest bottom line on the "99%" target
**Real-time 99% severe-weather reliability is NOT currently evidenced anywhere.** We keep 99%
as a *north-star*, but we design and report against **calibrated probabilistic skill**, not a
headline accuracy number. Here's why, with the data:

- The single "0.99" in the literature (SteamCast, arXiv:2503.22724) is **pixel-wise classification
  accuracy on ONE city (Yan'an, China), ONE variable (radar reflectivity)**, for 30-min hail
  nowcasts at 6-min intervals. Its companion *skill* score is only **ETS 0.18** — and an ablation
  shows performance collapses to MSE 0.40 / SSIM 0.43 without position+time embeddings (vs MSE 0.02
  / SSIM 0.81 with them). So 0.99 ACC is **not** a robust, generalizable reliability measure.
  - Source: https://arxiv.org/pdf/2503.22724
- The operationally serious benchmark — NOAA's **Warn-on-Forecast System (WoFS)** — trains random
  forests / gradient-boosted trees / logistic regression to predict tornado/severe-hail/severe-wind
  for 0–3 h forecasts (WoFS ensemble input every 5 min out to 150 min lead; 2017–19 HWT Spring
  Forecasting Experiments, 81 dates). It reports **better discrimination and more reliable
  probabilities than tuned-threshold baselines — not near-100% accuracy.**
  - Source (Monthly Weather Review 149(5):1535–1557, 2021, DOI 10.1175/MWR-D-20-0194.1):
    https://par.nsf.gov/biblio/10422706-using-machine-learning-generate-storm-scale-probabilistic-guidance-severe-weather-hazards-warn-forecast-system

**Design mandate:** target calibrated probabilistic reliability + honest skill metrics
(ETS, reliability diagrams, Brier score, lead time). Never ship a "99% accurate" claim.

---

## The credible aspirational path (how we chase very-high reliability anyway)
Three legs, combined — each realistic on its own, novel in combination:

### A. Symbolic constraint reasoning (MeTTa / Hyperon)
Encode atmospheric/physical constraints and fuse multiple model outputs + ensemble probabilities
into **interpretable, well-calibrated** hazard guidance — a symbolic layer over the WoFS-style
ML-over-ensemble approach that already beat tuned-threshold baselines on reliability.

### B. Probabilistic / thermodynamic compute for cheap on-device inference (Extropic-style)
Extropic's Thermodynamic Sampling Units (TSUs) sample from energy-based models / PGMs via Gibbs
sampling using **pbits** (transistor-only Bernoulli samplers), with each pbit's probability set by a
bias + weighted sum of *physically-close neighbors* — i.e. **fully distributed, neighbor-local
processing** (a natural match for a decentralized swarm). Claimed **orders of magnitude** less energy
for sampling; Denoising Thermodynamic Models "could be **10,000× more energy efficient**" than GPUs.
- **Honesty note:** the 10,000× figure is from *simulations of small TSU sections on small
  benchmarks*, not deployed weather models. Hardware today = XTR-0 proof-of-tech + open-source
  `thrml` Python lib (announced 2025-10-29). Treat as aspirational-but-real.
- Source: https://extropic.ai/writing/thermodynamic-computing-from-zero-to-one
- **Built (P5.4):** `thermodynamic.py` implements the *computation a TSU runs* — an Ising-style
  energy over the binary hazard field (each cell fits its ensemble evidence AND wants to agree with
  its 4 physically-close neighbors), sampled with **pbit Gibbs updates** (flip prob =
  `sigmoid(bias + neighbor coupling)`, exactly the TSU pbit rule). On a CPU this **denoises the
  per-cell ensemble speckle**: with gentle coupling it lowers Brier *and* the reliability penalty at
  every lead time while leaving threshold skill (ETS) roughly neutral (verified in tests). We claim
  **no energy number** — only the algorithm and its accuracy win are real today; the 10,000× is
  hardware-simulation, not ours.

### C. Sensor fusion + spatiotemporal embeddings
SteamCast's ablation proves position+time embeddings are decisive. Fuse radar + in-situ swarm
sensors (each TINA-X node is a mobile weather probe) + temporal context. Insect-inspired multimodal
fusion (visual+olfactory, see ../fly-biomimicry) is a proven low-power pattern.

> **Net TINA-X narrative:** *"Calibrated, energy-efficient, distributed probabilistic nowcasting that
> improves reliability and lead time"* — symbolic constraints + thermodynamic sampling + swarm
> sensor fusion. This is a genuinely novel stack (nobody has connected all three for nowcasting yet
> — that gap IS the opportunity).

## Open gaps (things we must build / prove)
- No source connects symbolic + thermodynamic + fusion into one working nowcasting system — **we build it.**
- Extropic TSUs have never been applied to nowcasting — **greenfield.**
- WoFS achievable skill numbers for 0–3 h hazard forecasts not captured — **benchmark ourselves.**
- Generalization across regions/seasons unproven — **multi-region validation required.**
