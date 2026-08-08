# Nzi Mission Control (React + TypeScript)

The landing + dashboard page hackathon judges / YC can open to *verify* Project Nzi is real:
live reflex telemetry, the two-rate-brain explanation, and the IoT bill of materials.

## Run it
```bash
# 1. Start the Rust API (in the repo root):
cargo run -p nzi-server            # serves http://127.0.0.1:8080

# 2. Start the frontend (in this folder):
npm install
npm run dev                        # opens the Vite dev server
```
The page works even without the server running (landing + how-it-works render; telemetry/BOM
show friendly "start the server" hints).

## Test / build
```bash
npm test        # Vitest: telemetry helpers, api mirror, BOM table render
npm run build   # type-check + production build
```

## Layout
- `src/api.ts` — TypeScript mirror of the Rust telemetry/BOM contract (keep in sync!).
- `src/telemetry.ts` — pure helpers (windowing, budget checks) — unit tested.
- `src/useTelemetry.ts` — WebSocket hook (auto-reconnect).
- `src/components/ReflexChart.tsx` — dependency-free SVG chart.
- `src/components/BomTable.tsx` — the IoT hardware table.
- `src/App.tsx` — the page.

## Contract note
`src/api.ts` field names MUST match `rust-core/src/telemetry.rs` and `rust-server/src/bom.rs`.
Change one side → change the other. Tests guard the shapes but not cross-language drift.
