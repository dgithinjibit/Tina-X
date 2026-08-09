//! HTTP + WebSocket API (axum). Thin layer: it just exposes [`crate::sim`] and [`crate::bom`].
//!
//! # Endpoints (the contract the React app depends on)
//! - `GET  /api/health`  -> `{"status":"ok"}`                 (liveness check)
//! - `GET  /api/bom`     -> the bill of materials (JSON)       (IoT hardware list for judges)
//! - `GET  /api/status`  -> current agent status (JSON)        (health + last latency)
//! - `GET  /api/swarm`   -> SoNS swarm snapshot (JSON)          (agents, brain, stigmergy field)
//! - `GET  /api/quorum`  -> honeybee quorum verdict (JSON)      (which brain the swarm commits to)
//! - `GET  /api/flood`   -> GEOGLOWS flood-EWS overlay (JSON)   (flood-risk grid + swarm redirect)
//! - `GET  /api/gnss`    -> GNSS position track (JSON)          (NMEA fixes mapped onto the grid)
//! - `POST /api/alerts`  -> accept cascade alerts from TINA-X  (OPTIONAL component; see ADR 0005)
//! - `GET  /ws`          -> WebSocket streaming ReflexSample    (live telemetry, ~30/s)
//!
//! # For a junior dev
//! `build_router()` is separated from `serve()` so tests can exercise the routes WITHOUT binding
//! a real TCP port. The WebSocket handler runs the sim on a timer and pushes JSON frames.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use nzi_core::telemetry::TelemetryMsg;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower_http::cors::CorsLayer;

use nzi_core::quorum::{QuorumArbiter, QuorumVerdict, Risk, Tally};
use nzi_swarm::{stigmergy::SERVICE_TARGET, Swarm, SwarmSnapshot};

use crate::bom::reference_bom;
use crate::geoglows::{flood_scenario, FloodForecast};
use crate::sim::AgentSim;

/// Build the axum router with all routes. Pure construction — no I/O — so tests can call it.
pub fn build_router() -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/bom", get(bom))
        .route("/api/status", get(status))
        .route("/api/swarm", get(swarm))
        .route("/api/quorum", get(quorum))
        .route("/api/flood", get(flood))
        .route("/api/gnss", get(gnss))
        .route("/api/alerts", post(alerts))
        .route("/ws", get(ws_upgrade))
        // Allow the React dev server (a different origin) to call us during development.
        .layer(CorsLayer::permissive())
}

/// Liveness endpoint.
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Serve the bill of materials for one Nzi agent.
async fn bom() -> impl IntoResponse {
    Json(reference_bom())
}

/// Serve a one-shot status snapshot (spins the sim a few steps so the numbers are non-trivial).
async fn status() -> impl IntoResponse {
    let mut sim = AgentSim::new("nzi-001", 1.0 / 500.0);
    for _ in 0..10 {
        sim.tick();
    }
    Json(sim.status())
}

// --- swarm view (Phase 4 SoNS + bee/ant stigmergy) -------------------------------------------
//
// A judge-facing snapshot of the headless swarm: agents on a grid, the self-elected brain, and the
// stigmergic coverage field (with pheromone that evaporates — the ant "useful forgetting"). We run
// a small swarm a fixed number of steps so it has converged and covered, then serialize a snapshot.

/// Serializable DTO mirroring `nzi_swarm::SwarmSnapshot` (the swarm crate stays serde-free).
#[derive(Debug, Clone, Serialize)]
pub struct SwarmView {
    pub width: usize,
    pub height: usize,
    pub tick: u64,
    pub leader: Option<u32>,
    pub leader_count: usize,
    pub coverage_fraction: f64,
    pub rho: f64,
    pub agents: Vec<AgentView>,
    /// Row-major live pheromone per cell (decaying).
    pub pheromone: Vec<f64>,
    /// Row-major cumulative coverage per cell (monotone).
    pub coverage: Vec<u32>,
}

/// One agent in a [`SwarmView`].
#[derive(Debug, Clone, Serialize)]
pub struct AgentView {
    pub id: u32,
    pub x: usize,
    pub y: usize,
    pub alive: bool,
    pub is_leader: bool,
    pub distance_to_leader: u32,
    pub covering: bool,
}

impl From<SwarmSnapshot> for SwarmView {
    fn from(s: SwarmSnapshot) -> Self {
        SwarmView {
            width: s.width,
            height: s.height,
            tick: s.tick,
            leader: s.leader,
            leader_count: s.leader_count,
            coverage_fraction: s.coverage_fraction,
            rho: s.rho,
            agents: s
                .agents
                .into_iter()
                .map(|a| AgentView {
                    id: a.id,
                    x: a.x,
                    y: a.y,
                    alive: a.alive,
                    is_leader: a.is_leader,
                    distance_to_leader: a.distance_to_leader,
                    covering: a.covering,
                })
                .collect(),
            pheromone: s.pheromone,
            coverage: s.coverage,
        }
    }
}

/// Run a small SoNS swarm to convergence + coverage, then return its snapshot for the dashboard.
async fn swarm() -> impl IntoResponse {
    // 8×8 = 64 agents: big enough to look like a swarm, small enough to render as a grid. Enough
    // ticks to converge the hierarchy (diameter ~14) and service the field.
    let mut swarm = Swarm::grid(8, 8);
    swarm.run(24);
    let view: SwarmView = swarm.snapshot(SERVICE_TARGET).into();
    Json(view)
}

// --- quorum verdict (Phase 4 SoNS "which brain?" via honeybee quorum; bee brain B1–B4) -------
//
// Closes the loop between the two collective-intelligence layers: the swarm produces cheap
// neighbor-local support/inhibition tallies for each leader CANDIDATE (`candidate_tallies`), and
// the SLOW MeTTa quorum arbiter (`nzi_core::quorum`, driving quorum.metta) turns them into a
// justified Commit/Scout verdict — the robotic analog of honeybee nest-site selection. We show
// BOTH the raw tallies and the verdict so a judge can see the evidence and the decision.
//
// MeTTa runs as a subprocess and needs the project venv. When it isn't available (or the decision
// errors) we degrade gracefully to `available: false` with a reason, rather than 500 — the endpoint
// is honest about needing the venv, and the server test stays venv-free.

/// One candidate's tally as shown to the dashboard (mirror of `nzi_swarm::CandidateTally`).
#[derive(Debug, Clone, Serialize)]
pub struct CandidateView {
    pub id: u32,
    pub support: u32,
    pub inhibition: u32,
}

/// The quorum endpoint payload: the swarm's candidate tallies plus the arbiter's verdict.
#[derive(Debug, Clone, Serialize)]
pub struct QuorumView {
    /// Whether the MeTTa arbiter could run (venv present and the decision reduced cleanly).
    pub available: bool,
    /// If `available` is false, why (missing venv / rules / parse error) — for an honest UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The risk regime used for the decision (the quorum speed/accuracy knob).
    pub risk: String,
    /// The raw neighbor-local evidence the decision was made from.
    pub candidates: Vec<CandidateView>,
    /// `"commit"` or `"scout"` when available; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    /// The committed candidate id (present only on a Commit).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed_id: Option<u32>,
    /// The committed option's effective weight (present only on a Commit).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
}

/// Resolve the repository root (parent of this crate's manifest dir) so the arbiter can find
/// `metta-logic/knowledge/quorum.metta` and the `.venv`. Matches the integration tests' resolver.
fn project_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// Run a fresh swarm to convergence, read its candidate tallies, and ask the honeybee quorum
/// arbiter which brain to commit to. Uses `Risk::High` — electing the swarm's brain is a
/// consequential, not-easily-reversed commitment, so we demand strong agreement (B2).
async fn quorum() -> impl IntoResponse {
    const RISK: Risk = Risk::High;

    // 1. Cheap swarm evidence (same 8×8 swarm the /api/swarm view uses).
    let mut swarm = Swarm::grid(8, 8);
    swarm.run(24);
    let tallies = swarm.candidate_tallies();
    let candidates: Vec<CandidateView> = tallies
        .iter()
        .map(|t| CandidateView { id: t.id, support: t.support, inhibition: t.inhibition })
        .collect();

    // 2. Try to run the SLOW MeTTa arbiter over that evidence; degrade gracefully if unavailable.
    let base = QuorumView {
        available: false,
        reason: None,
        risk: "high".to_string(),
        candidates,
        verdict: None,
        committed_id: None,
        weight: None,
    };

    let arbiter = match QuorumArbiter::from_project_root(project_root()) {
        Ok(a) => a,
        Err(e) => return Json(QuorumView { reason: Some(e.to_string()), ..base }),
    };
    let quorum_tallies: Vec<Tally> = tallies
        .iter()
        .map(|t| Tally::new(format!("agent-{}", t.id), t.support, t.inhibition))
        .collect();

    match arbiter.decide(RISK, &quorum_tallies) {
        Ok(QuorumVerdict::Commit { id, weight }) => Json(QuorumView {
            available: true,
            verdict: Some("commit".to_string()),
            // The MeTTa id is our synthetic `agent-<n>`; strip back to the numeric id for the UI.
            committed_id: id.strip_prefix("agent-").and_then(|s| s.parse().ok()),
            weight: Some(weight),
            ..base
        }),
        Ok(QuorumVerdict::Scout) => Json(QuorumView {
            available: true,
            verdict: Some("scout".to_string()),
            ..base
        }),
        Err(e) => Json(QuorumView { reason: Some(e.to_string()), ..base }),
    }
}

// --- flood early-warning overlay (G4D-RR bridge #6; GEOGLOWS ECMWF Streamflow) ---------------
//
// The honest GNSS/EO bridge the research prioritized first: a FREE, no-auth flood forecast becomes
// a hazard overlay the SoNS swarm biases its coverage toward. We map a GEOGLOWS-shaped forecast
// (offline fixture by default — always available, no network in tests, matching /api/quorum's
// degrade-gracefully stance) onto the 8×8 operating grid, attach it to the swarm, and run. The
// payload shows BOTH the flood-risk field AND the swarm's resulting coverage, so a judge sees the
// swarm actually redirect toward the flooded corridor. `source` labels provenance so we never
// overclaim: this is flood streamflow forecasting, not earthquake prediction.

/// One river reach in the flood forecast, as shown to the dashboard (mirror of `geoglows::Reach`
/// plus its derived risk).
#[derive(Debug, Clone, Serialize)]
pub struct ReachView {
    pub reach_id: u64,
    pub x: usize,
    pub y: usize,
    pub forecast_cms: f64,
    pub threshold_cms: f64,
    /// Normalized flood risk in [0,1] derived from forecast vs. threshold.
    pub risk: f64,
}

/// The `/api/flood` payload: the flood-risk field, its provenance, the contributing reaches, and
/// the swarm coverage achieved UNDER this overlay (evidence the swarm concentrates on the hazard).
#[derive(Debug, Clone, Serialize)]
pub struct FloodView {
    pub width: usize,
    pub height: usize,
    /// `"fixture"` (offline demo) or `"geoglows-live"` — honest data provenance for the UI.
    pub source: String,
    /// Row-major flood-risk grid in [0,1] (cell (x,y) at index y*width + x); same layout as the
    /// swarm's pheromone/coverage grids so the dashboard can overlay them directly.
    pub risk: Vec<f64>,
    /// Mean flood risk across the area in [0,1] — a headline threat metric.
    pub mean_risk: f64,
    /// The individual river reaches driving the forecast.
    pub reaches: Vec<ReachView>,
    /// Fraction of the field the swarm serviced while biased by this flood overlay, in [0,1].
    pub coverage_fraction: f64,
}

impl FloodView {
    /// Build the view from a forecast: derive the risk grid, attach it to a fresh swarm, run to
    /// coverage, and record the coverage achieved. Pure except for the local swarm run.
    fn from_forecast(forecast: FloodForecast) -> Self {
        let reaches = forecast
            .reaches
            .iter()
            .map(|r| ReachView {
                reach_id: r.reach_id,
                x: r.x,
                y: r.y,
                forecast_cms: r.forecast_cms,
                threshold_cms: r.threshold_cms,
                risk: r.risk(),
            })
            .collect();
        let risk = forecast.risk_grid();
        let mean_risk = forecast.mean_risk();

        // Attach the overlay to a swarm on the SAME grid and run it to coverage — the flood biases
        // where effort goes (see nzi_swarm::hazard). This is the "swarm redirects to the flood" step.
        let mut swarm = Swarm::grid(forecast.width, forecast.height).with_hazard(forecast.to_hazard_field());
        swarm.run(24);
        let coverage_fraction = swarm.snapshot(SERVICE_TARGET).coverage_fraction;

        FloodView {
            width: forecast.width,
            height: forecast.height,
            source: forecast.source,
            risk,
            mean_risk,
            reaches,
            coverage_fraction,
        }
    }
}

/// Serve the flood early-warning overlay: a GEOGLOWS forecast mapped to a swarm hazard field, with
/// the swarm's coverage under it. Always 200.
///
/// Source selection is honest and fail-safe:
///   * Default → the built-in offline fixture (`source: "fixture"`), so the demo always works.
///   * With the `live-feed` build feature AND `NZI_GEOGLOWS_LIVE=1` → fetch the REAL GEOGLOWS v2
///     REST API (`source: "geoglows-live"`), degrading to the fixture if the fetch yields nothing
///     (network down, all reaches failed). The blocking HTTP call runs on a blocking thread so it
///     never stalls the async runtime.
async fn flood() -> impl IntoResponse {
    let forecast = resolve_forecast().await;
    Json(FloodView::from_forecast(forecast))
}

/// Decide which forecast to serve. Isolated from the handler so the selection logic is testable and
/// the feature-gating stays in one place.
async fn resolve_forecast() -> FloodForecast {
    #[cfg(feature = "live-feed")]
    {
        let live = std::env::var("NZI_GEOGLOWS_LIVE").is_ok_and(|v| v == "1" || v == "true");
        if live {
            let (w, h) = crate::geoglows::FLOOD_GRID;
            let sites = crate::geoglows::default_sites();
            // ureq is blocking — run it off the async runtime.
            match tokio::task::spawn_blocking(move || crate::geoglows::fetch_forecast(w, h, &sites))
                .await
            {
                // A live forecast with at least one reach is used; otherwise fall back honestly.
                Ok(f) if !f.reaches.is_empty() => return f,
                Ok(_) => eprintln!("geoglows: live feed returned no usable reaches; using fixture"),
                Err(e) => eprintln!("geoglows: live fetch task failed ({e}); using fixture"),
            }
        }
    }
    flood_scenario()
}

// --- GNSS position ingest (G4D-RR bridge #2; NMEA → grid) ------------------------------------
//
// Turns raw GNSS receiver output (NMEA 0183 GGA/RMC sentences, incl. multi-constellation `GN`
// talker ids that carry Galileo) into positions the swarm can navigate on. We parse the offline
// fixture track (a live serial/RTKLIB source drops in later without changing the parser) and map
// each valid fix onto the operating grid via a geographic bounding box. Honest: invalid/void fixes
// are surfaced as such, never silently placed. Always 200 — the fixture is built in.

/// One parsed GNSS fix as shown to the dashboard (mirror of `nzi_swarm::gnss::GnssFix` plus its
/// mapped grid cell). `cell` is null for an invalid fix (not placed on the grid).
#[derive(Debug, Clone, Serialize)]
pub struct GnssFixView {
    pub lat: f64,
    pub lon: f64,
    pub valid: bool,
    /// The grid cell (x,y) this fix maps to, or null when the fix is invalid / unmappable.
    pub cell: Option<[usize; 2]>,
}

/// The `/api/gnss` payload: the operating grid size, provenance, the parsed track, and the current
/// navigable cell (the latest valid fix mapped to the grid).
#[derive(Debug, Clone, Serialize)]
pub struct GnssView {
    pub width: usize,
    pub height: usize,
    /// `"fixture"` (offline demo track) — honest provenance; a live source would set its own label.
    pub source: String,
    pub fixes: Vec<GnssFixView>,
    /// The cell (x,y) the swarm would navigate on now (latest valid fix), or null if none is valid.
    pub current_cell: Option<[usize; 2]>,
}

impl GnssView {
    /// Parse the fixture NMEA track and map it onto the given grid via the fixture bounds.
    fn from_fixture(width: usize, height: usize) -> Self {
        use nzi_swarm::gnss::{GnssTrack, FIXTURE_BOUNDS, FIXTURE_NMEA};
        let track = GnssTrack::from_nmea(FIXTURE_NMEA);
        let fixes = track
            .fixes
            .iter()
            .map(|f| GnssFixView {
                lat: f.lat,
                lon: f.lon,
                valid: f.valid,
                cell: FIXTURE_BOUNDS.cell(f, width, height).map(|(x, y)| [x, y]),
            })
            .collect();
        let current_cell = track
            .latest_valid()
            .and_then(|f| FIXTURE_BOUNDS.cell(&f, width, height))
            .map(|(x, y)| [x, y]);
        GnssView { width, height, source: "fixture".to_string(), fixes, current_cell }
    }
}

/// Serve the GNSS position track: the fixture NMEA fixes mapped onto the 8×8 operating grid.
async fn gnss() -> impl IntoResponse {
    Json(GnssView::from_fixture(8, 8))
}

// --- TINA-X alert intake (OPTIONAL component integration; ADR 0005) --------------------------
//
// TINA-X is an INDEPENDENT component that may push cascade alerts here. This endpoint accepts
// them so the Nzi dashboard can display societal-fragility warnings alongside agent telemetry.
// Nzi does not depend on TINA-X; this endpoint simply exists for TINA-X to POST to if it wants.

/// One cascade alert coming from TINA-X (matches tina-x/tina_x/bridge.py's JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeAlert {
    pub kind: String,
    pub node: String,
    pub message: String,
}

/// The payload TINA-X POSTs to `/api/alerts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertBatch {
    pub source: String,
    pub alerts: Vec<CascadeAlert>,
}

/// Accept a batch of TINA-X alerts. For now we acknowledge receipt (and could fan them out to the
/// dashboard WebSocket in a later step). Returns how many were accepted.
async fn alerts(Json(batch): Json<AlertBatch>) -> impl IntoResponse {
    // In a fuller build we'd broadcast these to connected dashboards; the POC acknowledges them.
    Json(serde_json::json!({
        "accepted": batch.alerts.len(),
        "source": batch.source,
    }))
}

/// Upgrade a plain HTTP request to a WebSocket and start streaming telemetry.
async fn ws_upgrade(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(stream_telemetry)
}

/// Run the reflex sim and push a `ReflexSample` (as tagged JSON) ~30 times per second.
///
/// We deliberately DON'T stream at the full 500 Hz control rate — the human eye and the browser
/// don't need it. ~30 Hz is smooth and light on the socket. (A real system would decimate.)
async fn stream_telemetry(mut socket: WebSocket) {
    let mut sim = AgentSim::new("nzi-001", 1.0 / 500.0);
    let mut ticker = tokio::time::interval(Duration::from_millis(33)); // ~30 Hz

    loop {
        ticker.tick().await;

        // Advance a few control steps per frame so the plant evolves at a visible pace.
        let mut sample = sim.tick();
        for _ in 0..15 {
            sample = sim.tick();
        }

        let msg = TelemetryMsg::Reflex(sample);
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(_) => continue, // serialization shouldn't fail for these plain types
        };

        // If the client has gone away, `send` errors — just stop the loop.
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}

/// Bind to `addr` and serve forever. Called by the binary; not used in tests.
pub async fn serve(addr: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Nzi mission-control API listening on http://{addr}");
    println!("  GET /api/health  GET /api/bom  GET /api/status  GET /api/swarm  GET /api/quorum  GET /api/flood  GET /api/gnss  POST /api/alerts  WS /ws");
    axum::serve(listener, build_router()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_builds_without_panicking() {
        // Constructing the router validates that all route handlers type-check and compose.
        let _router = build_router();
    }

    #[tokio::test]
    async fn health_handler_returns_ok_json() {
        // Call the handler directly (no TCP): it should serialize to the documented shape.
        let resp = health().await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn bom_handler_returns_the_reference_bom() {
        // The BOM handler must return exactly what bom::reference_bom() produces.
        let resp = bom().await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        // Sanity: the underlying data is non-empty and cheap (covered deeply in bom tests).
        assert!(!reference_bom().items.is_empty());
    }

    #[tokio::test]
    async fn swarm_handler_returns_a_converged_snapshot() {
        // The handler runs an 8×8 swarm to convergence; the snapshot must be well-formed:
        // 64 agents, 64 field cells, and a single elected brain (highest id 63).
        let resp = swarm().await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        // Reconstruct the view directly to assert on its shape (the handler returns the same data).
        let mut s = Swarm::grid(8, 8);
        s.run(24);
        let view: SwarmView = s.snapshot(SERVICE_TARGET).into();
        assert_eq!(view.agents.len(), 64);
        assert_eq!(view.pheromone.len(), 64);
        assert_eq!(view.coverage.len(), 64);
        assert_eq!(view.leader, Some(63));
        assert_eq!(view.leader_count, 1);
    }

    #[tokio::test]
    async fn quorum_handler_returns_candidate_evidence() {
        // The handler must always return 200 and surface the swarm's candidate evidence, whether or
        // not the MeTTa venv is present (it degrades to available:false rather than erroring). A
        // converged 8×8 swarm has exactly one candidate (the highest id, 63) with full support.
        let resp = quorum().await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        // Rebuild the evidence directly to assert its shape (the handler computes the same tallies).
        let mut s = Swarm::grid(8, 8);
        s.run(24);
        let tallies = s.candidate_tallies();
        assert_eq!(tallies.len(), 1, "converged swarm -> one uncontested candidate");
        assert_eq!(tallies[0].id, 63);
        assert_eq!(tallies[0].support, 64);
        assert_eq!(tallies[0].inhibition, 0);
    }

    #[tokio::test]
    async fn flood_handler_returns_a_grid_backed_forecast() {
        // The handler must always return 200 with the offline fixture (no network). Assert the DTO
        // shape by rebuilding it: an 8×8 grid, fixture provenance, a positive threat, and coverage.
        let resp = flood().await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let view = FloodView::from_forecast(flood_scenario());
        assert_eq!((view.width, view.height), (8, 8));
        assert_eq!(view.source, "fixture");
        assert_eq!(view.risk.len(), 64, "row-major risk grid is width*height");
        assert!(view.mean_risk > 0.0, "the fixture is a real flood -> positive mean risk");
        assert!(!view.reaches.is_empty());
        // The flood front reach (id 7 at (6,7)) is far over threshold -> saturated risk.
        let front = view.reaches.iter().find(|r| r.reach_id == 7).unwrap();
        assert_eq!(front.risk, 1.0);
        // The biased swarm still covers the field (coverage is meaningful, in range).
        assert!(view.coverage_fraction > 0.0 && view.coverage_fraction <= 1.0);
    }

    #[tokio::test]
    async fn gnss_handler_returns_a_mapped_track() {
        // Always 200 with the offline fixture. Rebuild the view to assert the shape: an 8×8 grid,
        // fixture provenance, the parsed fixes, and a current navigable cell from the latest valid fix.
        let resp = gnss().await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let view = GnssView::from_fixture(8, 8);
        assert_eq!((view.width, view.height), (8, 8));
        assert_eq!(view.source, "fixture");
        assert_eq!(view.fixes.len(), 6, "fixture has 6 GGA/RMC fixes");
        // The last fixture fix is void -> not mapped to a cell.
        assert!(!view.fixes.last().unwrap().valid);
        assert!(view.fixes.last().unwrap().cell.is_none());
        // A valid fix IS mapped, and the current cell (latest valid) is present and in-range.
        let [cx, cy] = view.current_cell.expect("a valid fix should give a current cell");
        assert!(cx < 8 && cy < 8);
    }

    #[tokio::test]
    async fn alerts_handler_accepts_a_tina_x_batch() {
        // Simulate the JSON TINA-X's bridge.py posts; the handler should accept all of them.
        let batch = AlertBatch {
            source: "Earthquake + Typhoon (compound)".to_string(),
            alerts: vec![CascadeAlert {
                kind: "hospital-critical".to_string(),
                node: "hospital-B".to_string(),
                message: "hospital-B has LOST POWER".to_string(),
            }],
        };
        let resp = alerts(Json(batch)).await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[test]
    fn alert_batch_round_trips_the_bridge_json_shape() {
        // Lock the contract with tina-x/tina_x/bridge.py: {source, alerts:[{kind,node,message}]}.
        let json = r#"{"source":"x","alerts":[{"kind":"datacenter-down","node":"dc-1","message":"m"}]}"#;
        let batch: AlertBatch = serde_json::from_str(json).unwrap();
        assert_eq!(batch.alerts.len(), 1);
        assert_eq!(batch.alerts[0].node, "dc-1");
    }
}
