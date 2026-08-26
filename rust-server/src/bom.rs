//! Bill of Materials (BOM) for one TINA-X agent — the IoT hardware, served to the dashboard.
//!
//! # For a junior dev / a hackathon judge
//! TINA-X is a *hardware* project (a fly-inspired robot), so people evaluating it want to see the
//! physical parts and rough cost, not just software. This module is the single source of truth
//! for that list. Each part is tied to a design decision from our research folders, so the BOM
//! doubles as a summary of WHY each component exists.
//!
//! Costs are ROUGH per-unit estimates (USD) for reasoning about a low-cost swarm agent; they are
//! not procurement quotes. The point is order-of-magnitude ("can a swarm be cheap?"), not cents.

use serde::{Deserialize, Serialize};

/// One line item in the bill of materials.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BomItem {
    /// Component name, e.g. "MEMS rate gyro (IMU)".
    pub name: String,
    /// What it does on the agent.
    pub purpose: String,
    /// Which fly trait / design decision it comes from (links to the research folders).
    pub rationale: String,
    /// Rough unit cost in USD (order-of-magnitude).
    pub est_cost_usd: f32,
    /// How many per agent.
    pub qty: u32,
}

impl BomItem {
    fn new(
        name: &str,
        purpose: &str,
        rationale: &str,
        est_cost_usd: f32,
        qty: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            purpose: purpose.to_string(),
            rationale: rationale.to_string(),
            est_cost_usd,
            qty,
        }
    }
}

/// The full BOM plus a computed total, ready to serialize to the frontend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bom {
    pub items: Vec<BomItem>,
    /// Sum of `est_cost_usd * qty` across all items.
    pub total_est_cost_usd: f32,
}

/// Build the reference BOM for a single low-cost TINA-X agent (prototype tier).
///
/// The design choices trace directly to `fly-biomimicry/` and `limitations-edge-cases/`:
/// cheap, collision-tolerant, optic-flow-guided, edge-compute — a deliberately inexpensive
/// node so a SWARM is affordable.
pub fn reference_bom() -> Bom {
    let items = vec![
        BomItem::new(
            "MEMS rate gyro / IMU",
            "Feeds the fast reflex loop (attitude-rate sensing).",
            "Fly halteres → low-latency rate feedback (fly-biomimicry/ #1).",
            3.00,
            1,
        ),
        BomItem::new(
            "Optic-flow sensor",
            "GPS-free obstacle avoidance & motion sensing.",
            "Fly compound-eye optic flow (fly-biomimicry/ #3).",
            5.00,
            1,
        ),
        BomItem::new(
            "Edge microcontroller (e.g. RP2040/ESP32-class)",
            "Runs the reflex loop; talks to the slow brain.",
            "Edge SWaP-C limits: no room for big models on-agent (limitations-edge-cases/ #4).",
            4.00,
            1,
        ),
        BomItem::new(
            "Brushless motors + ESCs",
            "Actuation.",
            "Small, cheap, replaceable — collision-tolerant design (fly-biomimicry/ #4).",
            6.00,
            4,
        ),
        BomItem::new(
            "Frame + self-righting protective cage",
            "Collision tolerance instead of expensive avoidance.",
            "GRASP-lab crash-and-recover approach (fly-biomimicry/ #4).",
            4.00,
            1,
        ),
        BomItem::new(
            "LiPo battery + power management",
            "Onboard power.",
            "Untethered power is the known hard frontier at small scale (fly-biomimicry/ caveat).",
            5.00,
            1,
        ),
        BomItem::new(
            "Environmental sensor (T/H/pressure, optional gas)",
            "Turns each agent into a mobile weather/enviro probe.",
            "Swarm-as-sensor-fleet for nowcasting (weather-prediction/).",
            3.00,
            1,
        ),
        BomItem::new(
            "Radio module (LoRa / 2.4 GHz)",
            "Neighbor-local swarm communication.",
            "SoNS self-organizing swarm uses local comms only (README, ADR-to-come).",
            4.00,
            1,
        ),
    ];

    let total = items.iter().map(|i| i.est_cost_usd * i.qty as f32).sum();
    Bom { items, total_est_cost_usd: total }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_is_non_empty_and_total_is_consistent() {
        let bom = reference_bom();
        assert!(!bom.items.is_empty());
        let recomputed: f32 = bom.items.iter().map(|i| i.est_cost_usd * i.qty as f32).sum();
        assert!((bom.total_est_cost_usd - recomputed).abs() < 1e-3);
    }

    #[test]
    fn bom_stays_cheap_enough_for_a_swarm() {
        // The whole thesis is "cheap agents so a swarm is affordable." Guard that intent:
        // if a change pushes a single agent over ~$100, we want a test to make us notice.
        let bom = reference_bom();
        assert!(
            bom.total_est_cost_usd < 100.0,
            "agent BOM too expensive for a swarm: ${}",
            bom.total_est_cost_usd
        );
    }

    #[test]
    fn every_item_cites_a_rationale() {
        // The BOM doubles as design documentation — every part must justify itself.
        for item in reference_bom().items {
            assert!(!item.rationale.trim().is_empty(), "{} lacks a rationale", item.name);
        }
    }
}
