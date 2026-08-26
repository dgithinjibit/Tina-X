//! Hazard field: a per-cell **flood-risk** overlay the swarm biases its coverage toward (G4D-RR
//! bridge #6; see `docs/research/g4drr-gnss-eo-bridge.md`). Where [`crate::stigmergy`] answers
//! *"has this cell been serviced recently?"*, the hazard field answers *"how much does this cell
//! NEED servicing?"* — the two combine so the swarm spends its effort where the disaster is.
//!
//! # Where the numbers come from
//! In a live system the values are driven by a real Earth-Observation feed: **GEOGLOWS ECMWF
//! Streamflow v2** (free, no-auth) gives a 15-day river-discharge forecast, which the server maps
//! onto the grid as a normalized flood-risk level per cell. This crate stays dependency-free and
//! feed-agnostic: it just holds the grid of risk levels; the `tina-server` `geoglows` module fills
//! it from the real feed (or an offline fixture). That mirrors how `stigmergy` stays pure while the
//! orchestrator drives it.
//!
//! # How it fuses with stigmergy (without breaking neighbor-locality)
//! An agent's coverage decision reads exactly ONE scalar — its own cell's level (see
//! `agent::Agent::consume`). We keep that invariant: the swarm passes an **effective** local level
//! = `pheromone − risk·bias`. A high-risk cell therefore reads as *less covered* than it physically
//! is, so agents keep servicing it longer and re-service it sooner as its trail fades. No agent ever
//! sees the global risk map — only its own cell's adjusted scalar, exactly as before.
//!
//! Risk is a normalized `f64` in `[0,1]` (0 = no flood risk, 1 = maximum). Out-of-range inputs are
//! clamped so a bad feed can never inject NaNs or negative bias into the coverage decision.

/// Default weight applied to a cell's flood risk when biasing the coverage decision. A cell at
/// full risk (1.0) reads as `bias` units *less* covered, so agents treat it as needing more work.
/// Tuned to be comparable to [`crate::stigmergy::COVERAGE_TARGET`] (3.0): at `bias = 3.0` a
/// max-risk cell is always considered under-covered → continuously serviced. This is a TUNED knob,
/// not "higher is better": too high starves low-risk cells, too low ignores the hazard.
pub const DEFAULT_HAZARD_BIAS: f64 = 3.0;

/// A grid of normalized flood-risk levels, row-major, matching a [`crate::stigmergy::CoverageField`]
/// of the same dimensions. Plain data — no serde, no I/O — so the swarm crate stays dependency-free.
#[derive(Clone, Debug)]
pub struct HazardField {
    width: usize,
    height: usize,
    /// Row-major flood-risk per cell, each in `[0,1]`. `risk[y*width + x]` is cell (x,y).
    risk: Vec<f64>,
    /// Weight applied to risk when computing the effective coverage level (see module docs).
    bias: f64,
}

impl HazardField {
    /// A field with no hazard anywhere (all cells risk 0), default bias.
    pub fn new(width: usize, height: usize) -> Self {
        Self::with_bias(width, height, DEFAULT_HAZARD_BIAS)
    }

    /// A no-hazard field with an explicit bias weight.
    pub fn with_bias(width: usize, height: usize, bias: f64) -> Self {
        Self { width, height, risk: vec![0.0; width * height], bias: bias.max(0.0) }
    }

    /// Build a field directly from a row-major risk grid (e.g. produced by the server's GEOGLOWS
    /// mapping). Values are clamped into `[0,1]`. Panics only if `levels.len() != width*height`,
    /// which is a programming error in the caller, not a runtime/feed condition.
    pub fn from_levels(width: usize, height: usize, mut levels: Vec<f64>, bias: f64) -> Self {
        assert_eq!(levels.len(), width * height, "risk grid must be width*height");
        for v in &mut levels {
            *v = v.clamp(0.0, 1.0);
        }
        Self { width, height, risk: levels, bias: bias.max(0.0) }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// This field's coverage-bias weight.
    pub fn bias(&self) -> f64 {
        self.bias
    }

    /// Normalized flood risk `[0,1]` at one cell.
    pub fn risk(&self, x: usize, y: usize) -> f64 {
        self.risk[y * self.width + x]
    }

    /// Set one cell's risk (clamped into `[0,1]`).
    pub fn set_risk(&mut self, x: usize, y: usize, level: f64) {
        self.risk[y * self.width + x] = level.clamp(0.0, 1.0);
    }

    /// The **effective** coverage level for a cell: the physical pheromone level reduced by this
    /// cell's flood risk (weighted by `bias`). This is the single scalar the swarm hands an agent,
    /// preserving neighbor-locality — a high-risk cell reads as under-covered so it gets serviced
    /// more. Never returns NaN (risk is clamped; pheromone is finite).
    pub fn effective_level(&self, x: usize, y: usize, pheromone: f64) -> f64 {
        pheromone - self.risk(x, y) * self.bias
    }

    /// The mean flood risk across the field, in `[0,1]` — a headline "how threatened is the whole
    /// area?" metric for the dashboard. Returns 0 for an empty field.
    pub fn mean_risk(&self) -> f64 {
        if self.risk.is_empty() {
            return 0.0;
        }
        self.risk.iter().sum::<f64>() / self.risk.len() as f64
    }

    /// How many cells are at or above a risk `threshold` — the "hot zone" size for the UI/alerts.
    pub fn cells_at_risk(&self, threshold: f64) -> usize {
        self.risk.iter().filter(|&&r| r >= threshold).count()
    }

    /// Row-major copy of the risk grid (for a snapshot/DTO). Cell (x,y) is index `y*width + x`.
    pub fn levels(&self) -> Vec<f64> {
        self.risk.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stigmergy::COVERAGE_TARGET;

    #[test]
    fn new_field_has_no_hazard() {
        let f = HazardField::new(3, 2);
        assert_eq!(f.mean_risk(), 0.0);
        assert_eq!(f.cells_at_risk(0.1), 0);
        // With zero risk, the effective level is just the pheromone (no bias applied).
        assert_eq!(f.effective_level(1, 1, 2.0), 2.0);
    }

    #[test]
    fn risk_is_clamped_into_unit_range() {
        let mut f = HazardField::new(2, 1);
        f.set_risk(0, 0, 5.0); // over-range feed value
        f.set_risk(1, 0, -3.0); // negative feed value
        assert_eq!(f.risk(0, 0), 1.0);
        assert_eq!(f.risk(1, 0), 0.0);
    }

    #[test]
    fn from_levels_clamps_and_maps_row_major() {
        // 2x2 grid, cell (1,0) index 1 over-range, cell (0,1) index 2 negative.
        let f = HazardField::from_levels(2, 2, vec![0.5, 9.0, -1.0, 0.25], 3.0);
        assert_eq!(f.risk(0, 0), 0.5);
        assert_eq!(f.risk(1, 0), 1.0);
        assert_eq!(f.risk(0, 1), 0.0);
        assert_eq!(f.risk(1, 1), 0.25);
    }

    #[test]
    fn high_risk_makes_a_covered_cell_read_as_under_covered() {
        // The core fusion property: a fully-covered cell (pheromone == COVERAGE_TARGET) that is also
        // at max flood risk must read BELOW target, so agents keep servicing it.
        let mut f = HazardField::with_bias(1, 1, DEFAULT_HAZARD_BIAS);
        f.set_risk(0, 0, 1.0);
        let eff = f.effective_level(0, 0, COVERAGE_TARGET);
        assert!(eff < COVERAGE_TARGET, "max-risk covered cell must read as under-covered");
    }

    #[test]
    fn zero_bias_disables_the_hazard_influence() {
        // bias = 0 recovers pure stigmergy: risk has no effect on the coverage decision. Useful as
        // an A/B baseline showing the hazard fusion is what changes behavior.
        let mut f = HazardField::with_bias(1, 1, 0.0);
        f.set_risk(0, 0, 1.0);
        assert_eq!(f.effective_level(0, 0, 2.0), 2.0);
    }

    #[test]
    fn mean_and_hot_zone_metrics() {
        let f = HazardField::from_levels(2, 2, vec![0.0, 1.0, 0.5, 0.5], 3.0);
        assert_eq!(f.mean_risk(), 0.5);
        assert_eq!(f.cells_at_risk(0.5), 3);
        assert_eq!(f.cells_at_risk(1.0), 1);
    }
}
