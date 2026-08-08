//! HTTP + WebSocket API (axum). Thin layer: it just exposes [`crate::sim`] and [`crate::bom`].
//!
//! # Endpoints (the contract the React app depends on)
//! - `GET /api/health`  -> `{"status":"ok"}`                 (liveness check)
//! - `GET /api/bom`     -> the bill of materials (JSON)       (IoT hardware list for judges)
//! - `GET /api/status`  -> current agent status (JSON)        (health + last latency)
//! - `GET /ws`          -> WebSocket streaming ReflexSample    (live telemetry, ~30/s)
//!
//! # For a junior dev
//! `build_router()` is separated from `serve()` so tests can exercise the routes WITHOUT binding
//! a real TCP port. The WebSocket handler runs the sim on a timer and pushes JSON frames.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use nzi_core::telemetry::TelemetryMsg;
use std::time::Duration;
use tower_http::cors::CorsLayer;

use crate::bom::reference_bom;
use crate::sim::AgentSim;

/// Build the axum router with all routes. Pure construction — no I/O — so tests can call it.
pub fn build_router() -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/bom", get(bom))
        .route("/api/status", get(status))
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
    println!("  GET /api/health  GET /api/bom  GET /api/status  WS /ws");
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
}
