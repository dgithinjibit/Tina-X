//! Project Nzi mission-control server library.
//!
//! Splits into:
//! - [`sim`]   — a testable live reflex simulation producing telemetry (no async).
//! - [`bom`]   — the IoT bill of materials, served to the dashboard.
//! - [`app`]   — the axum HTTP/WebSocket wiring (thin; logic lives in sim/bom).
//!
//! Keeping logic out of the async layer means almost everything is unit-testable without a
//! running server (see the tests in `sim` and `bom`).

pub mod app;
pub mod bom;
pub mod sim;
