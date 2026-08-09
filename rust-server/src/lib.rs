//! Project Nzi mission-control server library.
//!
//! Splits into:
//! - [`sim`]      — a testable live reflex simulation producing telemetry (no async).
//! - [`bom`]      — the IoT bill of materials, served to the dashboard.
//! - [`geoglows`] — GEOGLOWS flood-EWS feed → swarm hazard overlay (G4D-RR bridge #6; no async).
//! - [`app`]      — the axum HTTP/WebSocket wiring (thin; logic lives in sim/bom/geoglows).
//!
//! Keeping logic out of the async layer means almost everything is unit-testable without a
//! running server (see the tests in `sim`, `bom`, and `geoglows`).

pub mod app;
pub mod bom;
pub mod geoglows;
pub mod sim;
