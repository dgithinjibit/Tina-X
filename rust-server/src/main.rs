//! nzi-server binary — starts the mission-control API for the React dashboard.
//!
//! Run:  cargo run -p nzi-server
//! Then open the frontend (frontend/), which connects to http://127.0.0.1:8080.
//!
//! Address can be overridden with the NZI_ADDR env var, e.g. NZI_ADDR=0.0.0.0:9000.

use nzi_server::app;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Default to localhost:8080; allow an override so it can run in containers/hackathon boxes.
    let addr = std::env::var("NZI_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    app::serve(&addr).await
}
