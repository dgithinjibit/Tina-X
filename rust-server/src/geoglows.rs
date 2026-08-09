//! GEOGLOWS flood Early-Warning feed → swarm hazard overlay (G4D-RR bridge #6; see
//! `docs/research/g4drr-gnss-eo-bridge.md`).
//!
//! # What this is
//! GEOGLOWS ECMWF Streamflow v2 is a **free, no-auth** global river-discharge forecast (15-day,
//! 51-member ensemble, ~7M river reaches), available as a REST API (`geoglows.ecmwf.int`) and an
//! AWS Open Data S3 mirror (CC BY 4.0). It is the lowest-effort *real* GNSS/EO-adjacent feed to
//! wire end-to-end, and the research names it the first bridge to build.
//!
//! This module turns a set of river-reach forecasts into a normalized **flood-risk grid** that the
//! swarm consumes as a [`nzi_swarm::hazard::HazardField`], so coverage concentrates where flooding
//! is forecast. The mapping is pure, deterministic, and unit-testable — no async, no HTTP — exactly
//! like [`crate::sim`]. It mirrors the `/api/quorum` discipline: the endpoint always returns
//! something honest and offline (`source: "fixture"`), and a live fetch is an explicit opt-in that
//! degrades to the fixture if unavailable.
//!
//! # Honesty (per the research caveats)
//! This is FLOOD forecasting from hydrological streamflow — NOT earthquake prediction. The risk
//! level is a normalized proxy (forecast discharge vs. the reach's return-period threshold), not a
//! calibrated probability; the DTO labels its `source` so the dashboard never overclaims.

use nzi_swarm::hazard::{HazardField, DEFAULT_HAZARD_BIAS};

/// One river-reach streamflow forecast, mapped onto the coverage grid. Mirrors the fields we need
/// from a GEOGLOWS reach: a grid cell, the forecast peak discharge, and the reach's warning
/// threshold (e.g. a 2-year return-period flow). Plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct Reach {
    /// Reach id (GEOGLOWS "LINKNO"/comid in the real feed; synthetic in the fixture).
    pub reach_id: u64,
    /// Grid column this reach maps to.
    pub x: usize,
    /// Grid row this reach maps to.
    pub y: usize,
    /// Forecast peak discharge over the horizon (m³/s).
    pub forecast_cms: f64,
    /// Warning threshold discharge for this reach (m³/s) — the return-period flow above which the
    /// reach is considered in flood. `risk = clamp(forecast / threshold - 1, 0, 1)` roughly: at or
    /// below threshold → 0 risk; well above → saturates to 1.
    pub threshold_cms: f64,
}

impl Reach {
    /// Normalized flood risk in `[0,1]` for this reach: how far the forecast exceeds the warning
    /// threshold, saturating at 2× threshold. Below threshold is 0 (no warning). A zero/negative
    /// threshold is treated as "unknown" → 0 risk (never divides by zero or emits NaN).
    pub fn risk(&self) -> f64 {
        if self.threshold_cms <= 0.0 {
            return 0.0;
        }
        let ratio = self.forecast_cms / self.threshold_cms; // 1.0 == exactly at threshold
        (ratio - 1.0).clamp(0.0, 1.0)
    }
}

/// A grid-shaped flood forecast: the dimensions plus the reaches that fall on it.
#[derive(Debug, Clone)]
pub struct FloodForecast {
    pub width: usize,
    pub height: usize,
    /// Human/label describing where the data came from: `"fixture"` (offline demo scenario) or
    /// `"geoglows-live"` (a real fetch). Surfaced in the DTO so the UI is honest about provenance.
    pub source: String,
    pub reaches: Vec<Reach>,
}

impl FloodForecast {
    /// The row-major normalized risk grid: each cell is the MAX risk of any reach mapping to it
    /// (a cell floods if any river through it floods). Cells with no reach are 0.
    pub fn risk_grid(&self) -> Vec<f64> {
        let mut grid = vec![0.0f64; self.width * self.height];
        for r in &self.reaches {
            if r.x < self.width && r.y < self.height {
                let i = r.y * self.width + r.x;
                grid[i] = grid[i].max(r.risk());
            }
        }
        grid
    }

    /// Build the swarm hazard overlay from this forecast (default bias). This is the bridge: the
    /// real EO feed becomes the field the SoNS swarm biases its coverage toward.
    pub fn to_hazard_field(&self) -> HazardField {
        HazardField::from_levels(self.width, self.height, self.risk_grid(), DEFAULT_HAZARD_BIAS)
    }

    /// Mean forecast risk across the grid (headline "how flooded is the area?" for the UI).
    pub fn mean_risk(&self) -> f64 {
        let grid = self.risk_grid();
        if grid.is_empty() {
            return 0.0;
        }
        grid.iter().sum::<f64>() / grid.len() as f64
    }
}

// --- live GEOGLOWS v2 REST client -----------------------------------------------------------
//
// The real feed (verified against the geoglows/geoglows-rest-api Flask source):
//   * GET https://geoglows.ecmwf.int/api/v2/forecast/<river_id>?format=json
//       -> { "datetime": [...], "flow_median": [...], "flow_uncertainty_lower/upper": [...] }
//     The forecast PEAK discharge is max(flow_median) over the horizon.
//   * GET https://geoglows.ecmwf.int/api/v2/returnperiods/<river_id>?format=json
//       -> { "return_periods": { "2": <cms>, "5": <cms>, "10": <cms>, ... } }
//     We take the 2-year return period as the flood WARNING threshold (a standard bankfull proxy).
//
// The PARSING is pure and always compiled (unit-tested against captured JSON, no network). Only the
// HTTP call itself is behind the `live-feed` feature, so the default build stays network-free.

/// Base URL of the GEOGLOWS v2 REST data service.
pub const GEOGLOWS_API_BASE: &str = "https://geoglows.ecmwf.int/api/v2";

/// The return period (years) used as a reach's flood WARNING threshold. The 2-year flood is the
/// conventional bankfull/warning level; above it a reach is considered spilling.
pub const WARNING_RETURN_PERIOD: &str = "2";

/// Parse the forecast PEAK discharge (max of `flow_median`) from a GEOGLOWS `/forecast` JSON body.
/// Returns `None` if the shape is missing/empty, so a bad response degrades rather than panics.
pub fn parse_forecast_peak_cms(body: &serde_json::Value) -> Option<f64> {
    let series = body.get("flow_median")?.as_array()?;
    series
        .iter()
        .filter_map(|v| v.as_f64())
        .fold(None, |acc, x| Some(acc.map_or(x, |a: f64| a.max(x))))
}

/// Parse the flood warning threshold (the [`WARNING_RETURN_PERIOD`]-year return period, in m³/s)
/// from a GEOGLOWS `/returnperiods` JSON body. The return periods live under `return_periods` keyed
/// by year-as-string. `None` if absent, so a missing threshold degrades to zero-risk (never NaN).
pub fn parse_return_period_threshold(body: &serde_json::Value) -> Option<f64> {
    let rps = body.get("return_periods")?;
    // The value may be a number or a stringified number depending on formatter; handle both.
    let v = rps.get(WARNING_RETURN_PERIOD)?;
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// A river reach to fetch live, mapped to a grid cell. The caller supplies which real GEOGLOWS
/// `river_id` (LINKNO) corresponds to which cell of the operating area.
#[derive(Debug, Clone, PartialEq)]
pub struct ReachSite {
    pub river_id: u64,
    pub x: usize,
    pub y: usize,
}

/// Fetch one reach's forecast peak + warning threshold from the live GEOGLOWS v2 API and build a
/// [`Reach`]. Blocking (ureq). Returns a human-readable error string on any failure (the server
/// crate doesn't carry anyhow). Only compiled with the `live-feed` feature.
#[cfg(feature = "live-feed")]
pub fn fetch_reach(site: &ReachSite) -> Result<Reach, String> {
    let forecast_url = format!("{GEOGLOWS_API_BASE}/forecast/{}?format=json", site.river_id);
    let rp_url = format!("{GEOGLOWS_API_BASE}/returnperiods/{}?format=json", site.river_id);

    let fetch_json = |url: &str, what: &str| -> Result<serde_json::Value, String> {
        ureq::get(url)
            .call()
            .map_err(|e| format!("GEOGLOWS {what} request failed for river {}: {e}", site.river_id))?
            .body_mut()
            .read_json::<serde_json::Value>()
            .map_err(|e| format!("parsing GEOGLOWS {what} JSON for river {}: {e}", site.river_id))
    };

    let forecast_body = fetch_json(&forecast_url, "forecast")?;
    let rp_body = fetch_json(&rp_url, "returnperiods")?;

    let forecast_cms = parse_forecast_peak_cms(&forecast_body)
        .ok_or_else(|| format!("no flow_median in forecast for river {}", site.river_id))?;
    // A missing threshold → 0.0, which `Reach::risk` treats as "unknown" → 0 risk (safe default).
    let threshold_cms = parse_return_period_threshold(&rp_body).unwrap_or(0.0);

    Ok(Reach { reach_id: site.river_id, x: site.x, y: site.y, forecast_cms, threshold_cms })
}

/// Fetch a full flood forecast for a set of reach sites from the live GEOGLOWS API, assembled onto
/// a `width`×`height` grid. Any reach that fails to fetch is skipped (logged by the caller); the
/// forecast is still returned so one bad reach never sinks the whole overlay. `source` is set to
/// `"geoglows-live"` so the UI reports honest provenance. Only compiled with the `live-feed` feature.
#[cfg(feature = "live-feed")]
pub fn fetch_forecast(width: usize, height: usize, sites: &[ReachSite]) -> FloodForecast {
    let reaches = sites
        .iter()
        .filter_map(|s| match fetch_reach(s) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("geoglows: skipping river {}: {e}", s.river_id);
                None
            }
        })
        .collect();
    FloodForecast { width, height, source: "geoglows-live".to_string(), reaches }
}

/// An offline, deterministic flood scenario for demos and tests — a river system in flood cutting
/// across an 8×8 operating area (matching the `/api/swarm` grid). Anchored on the research's
/// disaster framing: a swollen main channel (the diagonal reaches) plus tributaries, so the swarm
/// is visibly pulled toward the flooded corridor. Values are illustrative m³/s, not a real gauge.
pub fn flood_scenario() -> FloodForecast {
    // A diagonal "river" of reaches across the 8×8 grid, discharge rising downstream and spilling
    // over threshold in the lower reaches (the flood front). Thresholds are constant per reach so
    // the risk ramp is easy to read: upstream at/below threshold (safe), downstream well over.
    let reaches = vec![
        Reach { reach_id: 1, x: 0, y: 1, forecast_cms: 40.0, threshold_cms: 100.0 }, // upstream, safe
        Reach { reach_id: 2, x: 1, y: 2, forecast_cms: 90.0, threshold_cms: 100.0 }, // rising, still safe
        Reach { reach_id: 3, x: 2, y: 3, forecast_cms: 120.0, threshold_cms: 100.0 }, // 0.2 over
        Reach { reach_id: 4, x: 3, y: 4, forecast_cms: 150.0, threshold_cms: 100.0 }, // 0.5 over
        Reach { reach_id: 5, x: 4, y: 5, forecast_cms: 190.0, threshold_cms: 100.0 }, // 0.9 over
        Reach { reach_id: 6, x: 5, y: 6, forecast_cms: 260.0, threshold_cms: 100.0 }, // saturated (flood front)
        Reach { reach_id: 7, x: 6, y: 7, forecast_cms: 300.0, threshold_cms: 100.0 }, // saturated
        // A tributary joining the flooded corridor from the side.
        Reach { reach_id: 8, x: 4, y: 6, forecast_cms: 175.0, threshold_cms: 100.0 }, // 0.75 over
    ];
    FloodForecast { width: 8, height: 8, source: "fixture".to_string(), reaches }
}

/// The grid dimensions the flood overlay uses (matches the `/api/swarm` 8×8 operating area).
pub const FLOOD_GRID: (usize, usize) = (8, 8);

/// Real GEOGLOWS reach sites to fetch when the live feed is enabled, mapped to grid cells.
///
/// These are placeholder `river_id`s (LINKNO) laid out along the same diagonal corridor as
/// [`flood_scenario`], so the live overlay reads like the demo. Swap them for the actual LINKNOs of
/// the target operating area (find them at <https://data.geoglows.org>) before a real deployment —
/// the mapping is deliberately explicit and data-driven, not hardcoded hydrology.
pub fn default_sites() -> Vec<ReachSite> {
    vec![
        ReachSite { river_id: 760021611, x: 0, y: 1 },
        ReachSite { river_id: 760021612, x: 1, y: 2 },
        ReachSite { river_id: 760021613, x: 2, y: 3 },
        ReachSite { river_id: 760021614, x: 3, y: 4 },
        ReachSite { river_id: 760021615, x: 4, y: 5 },
        ReachSite { river_id: 760021616, x: 5, y: 6 },
        ReachSite { river_id: 760021617, x: 6, y: 7 },
        ReachSite { river_id: 760021618, x: 4, y: 6 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_is_zero_at_or_below_threshold_and_saturates_above() {
        let safe = Reach { reach_id: 1, x: 0, y: 0, forecast_cms: 50.0, threshold_cms: 100.0 };
        assert_eq!(safe.risk(), 0.0);
        let at = Reach { reach_id: 2, x: 0, y: 0, forecast_cms: 100.0, threshold_cms: 100.0 };
        assert_eq!(at.risk(), 0.0, "exactly at threshold -> no warning yet");
        let mid = Reach { reach_id: 3, x: 0, y: 0, forecast_cms: 150.0, threshold_cms: 100.0 };
        assert!((mid.risk() - 0.5).abs() < 1e-9);
        let over = Reach { reach_id: 4, x: 0, y: 0, forecast_cms: 500.0, threshold_cms: 100.0 };
        assert_eq!(over.risk(), 1.0, "far over threshold -> saturates at 1");
    }

    #[test]
    fn zero_threshold_is_treated_as_unknown_not_nan() {
        let r = Reach { reach_id: 1, x: 0, y: 0, forecast_cms: 200.0, threshold_cms: 0.0 };
        assert_eq!(r.risk(), 0.0);
        assert!(r.risk().is_finite());
    }

    #[test]
    fn risk_grid_takes_the_max_per_cell_and_is_row_major() {
        // Two reaches on the same cell -> the cell takes the higher risk. Row-major index y*w + x.
        let f = FloodForecast {
            width: 2,
            height: 2,
            source: "fixture".to_string(),
            reaches: vec![
                Reach { reach_id: 1, x: 1, y: 1, forecast_cms: 150.0, threshold_cms: 100.0 }, // 0.5
                Reach { reach_id: 2, x: 1, y: 1, forecast_cms: 200.0, threshold_cms: 100.0 }, // 1.0
            ],
        };
        let grid = f.risk_grid();
        assert_eq!(grid.len(), 4);
        assert_eq!(grid[0], 0.0); // (0,0) no reach
        assert_eq!(grid[3], 1.0); // (1,1) index 3 -> max(0.5, 1.0)
    }

    #[test]
    fn scenario_maps_to_a_hazard_field_the_swarm_can_use() {
        let f = flood_scenario();
        assert_eq!(f.source, "fixture");
        let hz = f.to_hazard_field();
        assert_eq!((hz.width(), hz.height()), (8, 8));
        // The flood front (reach 7 at (6,7), 3x over threshold) must be a max-risk cell.
        assert_eq!(hz.risk(6, 7), 1.0);
        // An upstream safe reach must be zero risk.
        assert_eq!(hz.risk(0, 1), 0.0);
        // There is genuine hazard somewhere -> mean risk is positive.
        assert!(f.mean_risk() > 0.0);
    }

    // --- live-feed JSON parsing (network-free: parse captured-shape bodies) -----------------

    #[test]
    fn parses_forecast_peak_from_flow_median_series() {
        // Mirror of GET /api/v2/forecast/<id>?format=json — peak = max(flow_median).
        let body = serde_json::json!({
            "river_id": 123456789u64,
            "datetime": ["2026-08-09T00:00:00+00:00", "2026-08-09T03:00:00+00:00", "2026-08-09T06:00:00+00:00"],
            "flow_median": [40.0, 260.0, 150.0],
            "flow_uncertainty_lower": [30.0, 200.0, 120.0],
            "flow_uncertainty_upper": [55.0, 320.0, 190.0]
        });
        assert_eq!(parse_forecast_peak_cms(&body), Some(260.0));
    }

    #[test]
    fn forecast_peak_is_none_when_series_missing_or_empty() {
        assert_eq!(parse_forecast_peak_cms(&serde_json::json!({})), None);
        assert_eq!(parse_forecast_peak_cms(&serde_json::json!({ "flow_median": [] })), None);
    }

    #[test]
    fn parses_two_year_return_period_as_threshold() {
        // Mirror of GET /api/v2/returnperiods/<id>?format=json — threshold = the 2-year flow.
        let body = serde_json::json!({
            "river_id": 123456789u64,
            "return_periods": { "2": 100.0, "5": 160.0, "10": 210.0, "25": 280.0 }
        });
        assert_eq!(parse_return_period_threshold(&body), Some(100.0));
    }

    #[test]
    fn return_period_threshold_accepts_stringified_numbers() {
        // Some formatter paths emit the value as a string; we must still parse it.
        let body = serde_json::json!({ "return_periods": { "2": "125.5" } });
        assert_eq!(parse_return_period_threshold(&body), Some(125.5));
    }

    #[test]
    fn return_period_threshold_is_none_when_absent() {
        assert_eq!(parse_return_period_threshold(&serde_json::json!({})), None);
        let no_two = serde_json::json!({ "return_periods": { "5": 160.0 } });
        assert_eq!(parse_return_period_threshold(&no_two), None);
    }

    #[test]
    fn parsed_forecast_and_threshold_compose_into_a_reach_risk() {
        // End-to-end (still network-free): the two parsed values feed Reach::risk exactly like the
        // live fetch would assemble them. peak 260 vs threshold 100 -> ratio 2.6 -> saturated risk.
        let f_body = serde_json::json!({ "flow_median": [40.0, 260.0, 150.0] });
        let rp_body = serde_json::json!({ "return_periods": { "2": 100.0 } });
        let peak = parse_forecast_peak_cms(&f_body).unwrap();
        let threshold = parse_return_period_threshold(&rp_body).unwrap();
        let reach = Reach { reach_id: 1, x: 0, y: 0, forecast_cms: peak, threshold_cms: threshold };
        assert_eq!(reach.risk(), 1.0);
    }
}
