//! Stigmergy: coordinating field coverage through the environment, not through a plan (P4.3),
//! now with **pheromone evaporation** — the "useful forgetting" that makes stigmergy adaptive (P5→
//! bee/ant brain upgrade, see `docs/research/bee-ant-brain-mechanisms.md`, mechanisms A1/A2).
//!
//! Stigmergy = agents coordinate by leaving marks in a shared environment that other agents read
//! locally (like ants and pheromone). Here the environment is the field grid; each cell carries a
//! **pheromone level** that agents reinforce (deposit) while covering and that **decays every tick**.
//! An agent covers its cell only while the local pheromone is under [`COVERAGE_TARGET`], so effort
//! spreads to gaps WITHOUT any central task-allocation — the mark IS the allocation.
//!
//! ## Why decay (the ACO ρ term)
//! Ant Colony Optimization formalizes the trail update as `τ ← (1-ρ)·τ + deposit`, ρ∈(0,1] the
//! evaporation rate. Dorigo & Stützle: evaporation "implements a useful form of forgetting,
//! favoring exploration"; with **ρ=0 the algorithm does not converge** (trails never fade, the
//! field locks in). Our earlier `CoverageField` was monotonic (`mark` only ever `+=1`) — i.e. the
//! broken ρ=0 case: a covered cell stayed "hot" forever and coverage could never re-flow when
//! conditions changed (re-infestation, a cell needing re-treatment, a downed agent's zone going
//! stale). Evaporation fixes exactly that and is the mechanism behind our own P4.4 resilience goal.
//!
//! Two views are kept, deliberately:
//!   * **pheromone** (decaying `f64`) — drives the LIVE coverage decision; this is the adaptive part.
//!   * **cumulative coverage count** (monotone `u32`) — an audit trail of total work ever done,
//!     used for progress metrics (`coverage_fraction`) that ask "was this cell serviced?", not
//!     "is it hot right now?".
//!
//! Crucially an agent reads only its OWN cell's pheromone (neighbor-local / location-local). The
//! whole-field views here are for the orchestrator and tests, never for an agent's decision.

/// Default per-tick evaporation rate ρ (fraction of pheromone lost each tick). ACO uses ρ∈(0,1];
/// small values (0.01–0.1) work, ρ=0 fails to converge. This is a TUNED parameter, not "higher is
/// better": too high erases the trail before it can coordinate, too low re-creates the lock-in
/// (see `docs/research/bee-ant-brain-mechanisms.md` A2). Swept empirically for real swarm sizes.
pub const DEFAULT_EVAPORATION: f64 = 0.05;

/// A grid of stigmergic state — the shared environment.
#[derive(Clone, Debug)]
pub struct CoverageField {
    width: usize,
    height: usize,
    /// Row-major **pheromone** levels (decaying). `pheromone[y*width + x]` is the live trail
    /// strength at cell (x,y): reinforced by [`deposit`](Self::deposit)/[`mark`](Self::mark),
    /// faded by [`evaporate`](Self::evaporate). This is what an agent reads to decide to cover.
    pheromone: Vec<f64>,
    /// Row-major **cumulative** coverage counts (monotone). Total times a cell was ever covered —
    /// the audit/progress view, never decays.
    coverage: Vec<u32>,
    /// This field's evaporation rate ρ (per tick).
    rho: f64,
}

impl CoverageField {
    /// A fresh, entirely-uncovered field with the default evaporation rate.
    pub fn new(width: usize, height: usize) -> Self {
        Self::with_evaporation(width, height, DEFAULT_EVAPORATION)
    }

    /// A fresh field with an explicit evaporation rate ρ (clamped into [0,1]). ρ=0 recovers the old
    /// monotonic (non-adaptive) behavior — useful only for A/B tests demonstrating why decay matters.
    pub fn with_evaporation(width: usize, height: usize, rho: f64) -> Self {
        Self {
            width,
            height,
            pheromone: vec![0.0; width * height],
            coverage: vec![0; width * height],
            rho: rho.clamp(0.0, 1.0),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// This field's evaporation rate ρ.
    pub fn rho(&self) -> f64 {
        self.rho
    }

    /// Live **pheromone** level of one cell — the only read an agent is allowed to make (its own
    /// cell). This is what fades, so an agent re-covers a cell whose trail has decayed below target.
    pub fn level(&self, x: usize, y: usize) -> f64 {
        self.pheromone[y * self.width + x]
    }

    /// Cumulative times a cell has ever been covered (monotone audit view; does not decay).
    pub fn coverage_count(&self, x: usize, y: usize) -> u32 {
        self.coverage[y * self.width + x]
    }

    /// Deposit `amount` of pheromone at a cell (ant trail reinforcement). Also bumps the cumulative
    /// coverage count. Kept separate from [`evaporate`](Self::evaporate) so the ACO
    /// deposit-then-evaporate order is explicit at the call site.
    pub fn deposit(&mut self, x: usize, y: usize, amount: f64) {
        let i = y * self.width + x;
        self.pheromone[i] += amount;
        self.coverage[i] = self.coverage[i].saturating_add(1);
    }

    /// Mark one cell as covered once: a unit pheromone deposit (an agent covering its cell this
    /// tick). Thin wrapper over [`deposit`](Self::deposit) preserving the old call site.
    pub fn mark(&mut self, x: usize, y: usize) {
        self.deposit(x, y, 1.0);
    }

    /// Evaporate the whole field by one tick: `τ ← (1-ρ)·τ` for every cell (the ACO decay). Called
    /// once per swarm tick. This is the "useful forgetting" that lets coverage re-flow to gaps and
    /// re-service cells that have gone stale. Cumulative `coverage` is untouched (it's the audit view).
    pub fn evaporate(&mut self) {
        let keep = 1.0 - self.rho;
        for c in &mut self.pheromone {
            *c *= keep;
        }
    }

    /// How many cells currently hold at least `target` pheromone (the LIVE "hot" set). Because
    /// pheromone decays, this reflects *recent* coverage, not all-time. Orchestrator/test view.
    pub fn hot_cells(&self, target: f64) -> usize {
        self.pheromone.iter().filter(|&&c| c >= target).count()
    }

    /// How many cells have EVER been covered at least `target` times (cumulative). This is the
    /// honest "did the swarm service the field?" progress metric P4.4 checks — it must not decay,
    /// or a swarm that covered everything once would appear to lose coverage purely from evaporation.
    pub fn covered_cells(&self, target: u32) -> usize {
        self.coverage.iter().filter(|&&c| c >= target).count()
    }

    /// Fraction of the field ever covered to `target` (cumulative), in [0,1]. The headline
    /// "did the swarm cover the field?" metric (P4.4 resilience checks this recovers after loss).
    pub fn coverage_fraction(&self, target: u32) -> f64 {
        if self.coverage.is_empty() {
            return 1.0;
        }
        self.covered_cells(target) as f64 / self.coverage.len() as f64
    }

    /// Total cumulative marks laid down across the whole field (a proxy for total effort spent).
    /// Used to show stigmergy avoids gross over-treatment vs. a blanket policy.
    pub fn total_marks(&self) -> u32 {
        self.coverage.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_accumulate_per_cell() {
        // ρ=0 so pheromone doesn't decay within the test — isolates the deposit behavior.
        let mut f = CoverageField::with_evaporation(3, 2, 0.0);
        assert_eq!(f.level(1, 1), 0.0);
        f.mark(1, 1);
        f.mark(1, 1);
        assert_eq!(f.level(1, 1), 2.0);
        assert_eq!(f.level(0, 0), 0.0);
    }

    #[test]
    fn coverage_fraction_tracks_progress() {
        let mut f = CoverageField::new(2, 1); // two cells
        assert_eq!(f.coverage_fraction(1), 0.0);
        f.mark(0, 0);
        assert_eq!(f.coverage_fraction(1), 0.5);
        f.mark(1, 0);
        assert_eq!(f.coverage_fraction(1), 1.0);
    }

    #[test]
    fn covered_cells_respects_target() {
        let mut f = CoverageField::new(2, 1);
        f.mark(0, 0);
        f.mark(0, 0);
        f.mark(0, 0);
        assert_eq!(f.covered_cells(3), 1);
        assert_eq!(f.covered_cells(4), 0);
        assert_eq!(f.total_marks(), 3);
    }

    #[test]
    fn evaporation_fades_pheromone_but_not_cumulative_coverage() {
        // The core A1/A2 property: pheromone decays each tick, coverage count does not.
        let mut f = CoverageField::with_evaporation(1, 1, 0.5);
        f.mark(0, 0);
        assert_eq!(f.level(0, 0), 1.0);
        assert_eq!(f.coverage_count(0, 0), 1);
        f.evaporate();
        assert_eq!(f.level(0, 0), 0.5, "pheromone halved by ρ=0.5");
        f.evaporate();
        assert_eq!(f.level(0, 0), 0.25);
        assert_eq!(f.coverage_count(0, 0), 1, "cumulative coverage must NOT decay");
    }

    #[test]
    fn zero_evaporation_recovers_monotonic_behavior() {
        // ρ=0 is the OLD broken case: pheromone never fades. Kept as an explicit A/B baseline.
        let mut f = CoverageField::with_evaporation(1, 1, 0.0);
        f.mark(0, 0);
        f.evaporate();
        f.evaporate();
        assert_eq!(f.level(0, 0), 1.0, "ρ=0 → no forgetting (the non-adaptive case)");
    }

    #[test]
    fn a_stale_cell_becomes_re_coverable_after_enough_evaporation() {
        // Adaptivity: a cell covered to target eventually falls back under target as its trail
        // fades, so an agent would re-service it — the behavior the monotonic field could never show.
        let mut f = CoverageField::with_evaporation(1, 1, 0.3);
        for _ in 0..(COVERAGE_TARGET.ceil() as u32) {
            f.mark(0, 0);
        }
        assert!(f.level(0, 0) >= COVERAGE_TARGET as f64, "freshly covered → hot");
        // Let it decay untended for many ticks.
        for _ in 0..30 {
            f.evaporate();
        }
        assert!(
            f.level(0, 0) < COVERAGE_TARGET as f64,
            "a long-untended cell must fade below target and become re-coverable"
        );
    }
}

/// Desired **pheromone** level a cell should hold before agents stop actively covering it. With
/// evaporation, a cell drops back under this once its trail fades — which is what makes coverage
/// re-flow to stale gaps. Small constant; coverage is driven by local pheromone, not a global plan.
pub const COVERAGE_TARGET: f64 = 3.0;

/// Desired **cumulative** number of times a cell should be serviced before we call the field
/// "covered" for progress metrics ([`CoverageField::coverage_fraction`]). This is the monotone
/// audit target — distinct from the decaying pheromone [`COVERAGE_TARGET`] that gates live effort.
pub const SERVICE_TARGET: u32 = 3;
