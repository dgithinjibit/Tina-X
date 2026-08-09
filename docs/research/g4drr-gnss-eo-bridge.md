# G4D-RR Deep-Research Report — Bridging TINA-X to a real GNSS/EO layer

> **Provenance.** Produced by the `deep-research` workflow (run `wf_2bfa6d7d-3a4`): 5 search angles → parallel WebSearch → URL-deduped fetch → 3-vote adversarial verification (2/3 refutes kills a claim) → synthesis. 106 of 107 agents completed; the final synthesis agent had all 25 confirmed claims but could not emit them through a length-limited structured-output wrapper, so this report was reconstructed verbatim from its transcript. Every finding below survived adversarial verification (votes noted).
>
> **Target.** GNSS 4 Disaster Risk Reduction (G4D-RR) hackathon — SATNAV Africa JPO / DoraHacks. Submission window **2026-08-20 → 2026-09-05**. Virtual, Africa-focused. Prize: SpaceSTAR 2026 (Tunis).
> Judging (each /10): Originality, Sustainability, Significance, Applicability & Transferability, Market Potential, Impact.

---

## (a) Recommended track — DECIDED

**HYBRID, Track I-led** — **Track I (Drones for Emergency Applications: EWS / SAR / DRRM)** as the spine, with a **Track II (Environmental Monitoring)** sensing layer bolted on.

**Why (rubric-justified):** TINA-X's strongest differentiators — the fail-closed **verification moat**, **ZK safe-to-fly** attestation, the **self-healing decentralized swarm**, and **honest calibrated nowcasting** — all score hardest on *emergency autonomy* (Originality, Impact, Significance, trust). Adding a GNSS-R / EO sensing input buys the *Applicability & Transferability* breadth the rubric rewards without diluting the emergency-response narrative. Pure Track II would waste the swarm/verify/ZK story; pure Track I would leave the "Environmental Monitoring" half of the rubric unscored. Hybrid captures both.

---

## (b) The buildable-by-Sep-5 GNSS/EO bridges (ranked by leverage)

Each names a **real data source + real library + the existing TINA-X component it extends + the criteria it lifts**. All free/open. Caveats are honest and load-bearing.

### 1. NAV BRIDGE — Galileo HAS + OSNMA  · confidence: high · votes 3-0 / 3-0 / 3-0 / 3-0
Galileo **HAS** (High Accuracy Service) gives **free, open ~20 cm real-time PPP corrections** via the E6-B signal *and* the internet. **OSNMA** gives **free anti-spoofing authentication** of Open Service navigation data. Together → swarm navigation that is both **accurate and verifiable**, feeding the ZK safe-to-fly attestation.
- **Evidence:** HAS is open-access/free, providing orbit/clock/bias corrections via E6-B + internet; **SL1** targets global 20 cm H / 40 cm V (95%) @ 300 s convergence; **SL2** adds atmospheric corrections **Europe-only** @ 100 s. OSNMA authenticates I/NAV ephemeris/timing, mitigating spoofing "potentially redirecting the trajectory of aircraft, ships or drones"; **Initial Service operational July 2025** — free and usable now.
- **CAVEAT (honest):** HAS Initial-Service convergence is **~25–60 min in practice**, not the 300 s target. SL2 atmospheric corrections are **Europe-only**, so **over Africa expect SL1-grade global PPP**.
- **Extends:** `rust-swarm` navigation + `zk-rust` safe-to-fly (**OSNMA authentication status becomes a ZK witness** — "positioned by authenticated signals").
- **Lifts:** Originality, Impact, Significance, Applicability.
- **Sources:** https://www.gsc-europa.eu/sites/default/files/sites/all/files/Galileo-HAS-SDD_v1.0.pdf · https://www.euspa.europa.eu/galileo-osnma

### 2. INGEST-ENGINE BRIDGE — RTKLIB  · confidence: high · votes 3-0 / 3-0 / 3-0
**RTKLIB** is the concrete free, open-source engine that turns raw GNSS into positions. Supports **Galileo** (+ GPS/GLONASS/QZSS/BeiDou/SBAS), both **RTK and PPP**, and every interchange format a Rust/Python client needs.
- **Evidence:** Modes — Single / DGPS / Kinematic / Static / Moving-Baseline / Fixed / PPP-Kinematic / PPP-Static / PPP-Fixed (real-time + post-processing). Formats — RTCM 2.3/3.1/3.2, BINEX, NTRIP 1.0, NMEA 0183, SP3-c, ANTEX 1.4, IONEX 1.0, u-blox/NovAtel raw.
- **CAVEAT:** RTKLIB is **C** → bridge via subprocess/FFI (consistent with the project's existing subprocess-bridge decision). It does **not** natively ingest the HAS correction stream; real-time RTK needs live base-station data.
- **Extends:** `rust-swarm` navigation input layer.
- **Lifts:** Applicability & Transferability, Market Potential.
- **Source:** https://github.com/tomojitakasu/RTKLIB/blob/master/readme.txt

### 3. SAR BRIDGE — Galileo SAR / Return Link Service  · confidence: high · votes 3-0 / 3-0
**Galileo SAR** (Europe's MEOSAR contribution to COSPAS-SARSAT) relays **406 MHz distress beacons**; its **Return Link Service (RLS)** auto-acknowledges that a beacon's distress signal was received and position computed — a real operational hook for TINA-X **SAR drone tasking**.
- **Evidence:** SAR/Galileo reached **FOC Oct 2024** (27 sats carry SAR payloads); **RLS operational Jan 2020**, ~37 s average acknowledgement. Drones can be tasked to beacon coordinates to localize trapped people (directly addresses the Kumamoto comms/localization failure).
- **CAVEAT:** TINA-X **consumes** beacon positions; it does **not** operate the MEOSAR ground segment.
- **Extends:** `rust-swarm` quorum target selection (`rust-core/src/quorum.rs`).
- **Lifts:** Significance, Impact.
- **Source:** https://www.gsc-europa.eu/galileo/services/search-and-rescue-sar-galileo-service

### 4. GNSS-R SENSING BRIDGE (Track II spine) — CYGNSS / Spire / HydroGNSS  · confidence: high
GNSS-**Reflectometry** is real remote **SENSING** (not positioning): soil moisture, flood/inundation extent, wetlands — a new observation input to the nowcasting pipeline and the Track II environmental layer.
- **Evidence:** CYGNSS **L3 SM v3.2** → 0–5 cm volumetric soil moisture at 9/36 km; maps flood inundation **finer than SMAP** (~7×0.5 km, 1–3 day repeat) — ~32,580 km² flooded (Hurricane Harvey), ~7,210 km² (Irma). **HydroGNSS** (launched Nov 2025) uses reflected L-band GPS/Galileo for soil moisture / inundation / wetlands. **Spire** GNSS-R tracks GPS/Galileo/BeiDou/QZSS, ubRMSD 0.062 m³/m³.
- **CAVEAT (Africa-relevant, honest):** CYGNSS covers only **~±38.15° latitude** — which *favors* most of Africa but excludes the far north/south. **HydroGNSS open data is ~mid-2026 → treat as emerging/future, not usable today.**
- **Extends:** `weather-prediction` nowcasting pipeline + swarm coverage.
- **Lifts:** Significance, Sustainability, Applicability, Originality.
- **Sources:** https://data.nasa.gov/dataset/cygnss-level-3-soil-moisture-version-3-2-1feb5 · https://www.nature.com/articles/s41598-018-27673-x · https://www.esa.int/Applications/Observing_the_Earth/FutureEO/HydroGNSS/ESA_s_HydroGNSS_mission_launched_to_scout_for_water · https://www.tandfonline.com/doi/full/10.1080/01431161.2023.2270108

### 5. GNSS-MET BRIDGE (highest-leverage nowcasting extension) — PWV / ZTD  · confidence: high
GNSS-derived **Precipitable Water Vapor / Zenith Tropospheric Delay** is a **proven, reproducible** short-range nowcasting input — a *new real observation* into the existing pipeline.
- **Evidence:** RF→LSTM using GNSS PWV gives 1–2 h precip forecast (R² 0.64); pipeline is RINEX → **MG-APP (free PPP)** → PWV from a single ground station; GNSS-ZTD into **WRF 3D-Var** improves precip up to 6 h (PWV RMSE up to 2.5 mm below control).
- **CAVEAT:** raises false-alarm rate, threshold-dependent → **keep Brier / reliability / ETS metrics, never claim "99%".**
- **Extends:** `weather-prediction` pipeline (a real observation input feeding the existing ensemble→swarm→thermodynamic→symbolic stack).
- **Lifts:** Significance, Sustainability, Originality.
- **Sources:** https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12563812/ · https://nhess.copernicus.org/articles/23/3319/2023/

### 6. FLOOD-EWS BRIDGE (lowest build effort, high demo value) — GEOGLOWS ECMWF Streamflow v2  · confidence: high
A **free, no-auth** global river-discharge forecast, ideal as the flood-EWS feed the swarm fuses. Fastest thing to wire end-to-end for a demo.
- **Evidence:** REST at `geoglows.ecmwf.int` **or** AWS Open Data S3 (no account, `--no-sign-request`, CC BY 4.0); **3-hourly, 51-member, 15-day** IFS ensemble over ~7M rivers.
- **CAVEAT:** REST is rate-limited (<50 rivers/day → use the S3 mirror for bulk); 3-hourly resolution only extends to day 6.
- **Extends:** `rust-swarm` stigmergy/quorum target selection + `SwarmPanel` (dashboard) + `/api` layer.
- **Lifts:** Applicability & Transferability, Market Potential, Impact.
- **Source:** https://geoglows.ecmwf.int/documentation

---

## (c) Kumamoto → GNSS-DR → TINA-X mapping table

2016 Kumamoto sequence (Apr 2016; Mw 6.2 foreshock + Mw 7.0 mainshock, Futagawa–Hinagu fault zone; Aso-area landslides). Generic failure modes → what GNSS-DR actually does → which TINA-X component maps.

| Kumamoto failure mode | GNSS-DR capability (real, not prediction) | TINA-X component that maps |
|---|---|---|
| Trapped-people **localization** & comms breakdown | SAR/Galileo 406 MHz beacon relay + RLS position ack | Bridge #3 → `rust-swarm` quorum tasks SAR drones to beacon coords |
| **Road/bridge access** for relief unknown post-quake | Sentinel-1 **InSAR** deformation + optical rapid-mapping (EO) | MeTTa brain consumes a deformation/blockage map → route proposals gated by the verify moat |
| **Compound aftershock** uncertainty | Honest, calibrated hazard nowcasting (NOT seismic prediction) | `weather-prediction`-style calibration discipline; report Brier/reliability, never certainty |
| Precise **UAV navigation** with degraded ground infra | Galileo HAS PPP (~20 cm) + OSNMA anti-spoof | Bridge #1 → `rust-swarm` nav + `zk-rust` safe-to-fly (OSNMA as ZK witness) |
| **Flood/landslide** follow-on hazard (Aso) | GNSS-R soil moisture/inundation + GEOGLOWS streamflow | Bridges #4/#6 → swarm coverage + flood-EWS fusion |
| Central-coordination fragility when infra is down | (architectural) decentralized self-healing swarm | SoNS leader election + ant stigmergy — the core Kumamoto resilience lesson |

**21st-century calamity scan (broadens the problem statement):** Tōhoku 2011 (GNSS-derived fault slip → tsunami EW is the canonical success), Hurricane Harvey/Irma 2017 (CYGNSS flood mapping, cited above), plus recurring African flood/drought contexts that GEOGLOWS + GNSS-R directly serve. Anchor narrative on Kumamoto; cite these as transferability evidence.

---

## (d) Honest caveats — where NOT to overclaim

- **GNSS does NOT predict earthquakes.** It does crustal-deformation monitoring, co-seismic displacement / GNSS-seismology, tsunami EW via GNSS-derived **fault slip**, and precise UAV navigation. Any "predict earthquakes with GNSS" framing is false — do not write it.
- **Nowcasting metrics stay honest.** Brier score, reliability diagram, ETS. GNSS-PWV *raises false-alarm rate* and is threshold-dependent. **Never "99% accuracy."** (Consistent with the project's existing weather-honesty stance.)
- **Emerging ≠ operational.** Galileo **EWSS is NOT operational** (EUSPA still procuring the ERAS component). **HydroGNSS open data ~mid-2026.** Frame both as future/emerging, not usable-today.
- **HAS reality over Africa:** SL1-grade global PPP, ~25–60 min real convergence today, SL2 atmospheric corrections Europe-only.
- **RTKLIB is C** — integrate via subprocess/FFI; it does not ingest the HAS stream natively; live RTK needs a base station (note African CORS reality: AFREF / TrigNet / regional networks).

---

## Realistic Sep-5 build plan — implement vs. narrate

**IMPLEMENT (working code + live demo):**
1. **GEOGLOWS flood-EWS ingest** (Bridge #6) — no auth, fastest path to a real GNSS/EO-adjacent feed the swarm fuses and the dashboard shows. Do this first.
2. **GNSS position ingest** (Bridge #2) — RTKLIB subprocess bridge → RINEX/NMEA → positions into `rust-swarm`. Use a public RINEX sample if no live receiver.
3. **GNSS-PWV nowcasting input** (Bridge #5) — one ground station → MG-APP → PWV as a new observation into `weather-prediction`; report calibrated metrics.
4. **OSNMA-status → ZK witness** (Bridge #1, thin slice) — thread an authentication-status flag into the `zk-rust` safe-to-fly witness even if HAS corrections are mocked.

**NARRATE (in the BUIDL writeup, with citations — real but not fully wired by Sep 5):**
- Galileo HAS full PPP correction stream (cite SDD; note convergence caveat).
- SAR/Galileo RLS drone tasking (cite service page; show the quorum hook).
- CYGNSS/HydroGNSS GNSS-R sensing layer (cite; flag HydroGNSS as mid-2026).
- Sentinel-1 InSAR post-quake deformation for the MeTTa brain.

**Packaging for a winning DoraHacks BUIDL:** demo video, live dashboard, reproducible repo, explicit dataset attribution (CC BY 4.0 etc.), the crisp Kumamoto→solution narrative above, an Africa deployment story (Zipline Rwanda/Ghana drone-corridor precedent; AFREF/TrigNet CORS + connectivity constraints), and a per-criterion pitch drawn from the "Lifts" tags on each bridge.

---

## Implementation status (bridge #6 built)

Bridge #6 (GEOGLOWS flood-EWS) is implemented end-to-end and both the offline and live paths are in the tree:

- **Swarm:** `rust-swarm/src/hazard.rs` — `HazardField` biases coverage toward flood cells (`effective_level = pheromone − risk·bias`), fused in `Swarm::step` without breaking neighbor-locality.
- **Server:** `rust-server/src/geoglows.rs` — maps a forecast to grid risk (`risk = clamp(forecast/threshold − 1, 0, 1)`, threshold = 2-year return period). Offline `flood_scenario()` fixture is the default. `GET /api/flood` → `FloodView`.
- **Live feed (real GEOGLOWS v2 REST):** behind the `live-feed` cargo feature (optional `ureq` dep). Endpoints verified against the `geoglows/geoglows-rest-api` source:
  - `GET https://geoglows.ecmwf.int/api/v2/forecast/<river_id>?format=json` → peak = `max(flow_median)`
  - `GET https://geoglows.ecmwf.int/api/v2/returnperiods/<river_id>?format=json` → threshold = `return_periods["2"]`
  - **Run it:** `NZI_GEOGLOWS_LIVE=1 cargo run -p nzi-server --features live-feed`. Degrades to the fixture on any network/parse failure; `source` field reports `"geoglows-live"` vs `"fixture"` honestly. Swap the placeholder LINKNOs in `geoglows::default_sites()` for the target area's real river ids (find at https://data.geoglows.org).
  - JSON parsing is network-free unit-tested (`parse_forecast_peak_cms`, `parse_return_period_threshold`); only the HTTP call is feature-gated.
