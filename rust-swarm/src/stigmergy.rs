//! Stigmergy: coordinating field coverage through the environment, not through a plan (P4.3).
//!
//! Stigmergy = agents coordinate by leaving marks in a shared environment that other agents read
//! locally (like ants and pheromone). Here the environment is the field grid; each cell counts how
//! many times it has been "covered" (inspected/treated). An agent covers its cell only while that
//! cell is under-covered ([`crate::agent::COVERAGE_TARGET`]), so effort spreads to gaps WITHOUT
//! any central task-allocation — the mark IS the allocation.
//!
//! Crucially an agent reads only [`CoverageField::level`] for its OWN cell (neighbor-local /
//! location-local). The whole-field views here are for the orchestrator and tests, never for an
//! agent's decision.

/// A grid of coverage counters — the shared stigmergic environment.
#[derive(Clone, Debug)]
pub struct CoverageField {
    width: usize,
    height: usize,
    /// Row-major mark counts. `cells[y*width + x]` = times cell (x,y) has been covered.
    cells: Vec<u32>,
}

impl CoverageField {
    /// A fresh, entirely-uncovered field.
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, cells: vec![0; width * height] }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Coverage count of one cell (the only read an agent is allowed to make — its own cell).
    pub fn level(&self, x: usize, y: usize) -> u32 {
        self.cells[y * self.width + x]
    }

    /// Mark one cell as covered once (an agent covering its cell this tick).
    pub fn mark(&mut self, x: usize, y: usize) {
        self.cells[y * self.width + x] += 1;
    }

    /// How many cells have reached at least `target` coverage. Orchestrator/test view of progress.
    pub fn covered_cells(&self, target: u32) -> usize {
        self.cells.iter().filter(|&&c| c >= target).count()
    }

    /// Fraction of the field that has reached `target` coverage, in [0,1]. The headline
    /// "did the swarm cover the field?" metric (P4.4 resilience checks this recovers after loss).
    pub fn coverage_fraction(&self, target: u32) -> f64 {
        if self.cells.is_empty() {
            return 1.0;
        }
        self.covered_cells(target) as f64 / self.cells.len() as f64
    }

    /// Total marks laid down across the whole field (a proxy for total effort spent). Used to show
    /// stigmergy avoids gross over-treatment: a blanket policy would keep marking covered cells.
    pub fn total_marks(&self) -> u32 {
        self.cells.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_accumulate_per_cell() {
        let mut f = CoverageField::new(3, 2);
        assert_eq!(f.level(1, 1), 0);
        f.mark(1, 1);
        f.mark(1, 1);
        assert_eq!(f.level(1, 1), 2);
        assert_eq!(f.level(0, 0), 0);
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
}
