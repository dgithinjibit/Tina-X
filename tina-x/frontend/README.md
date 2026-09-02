# TINA-X Frontend

> **Gen-X themed hackathon UI** for the TINA-X cascading failure reasoner — symbolic AI meets 90s cyberpunk aesthetics.

## Features

- 🧠 **Real-time cascade simulation** — select hazard scenarios, watch infrastructure fail
- 🕸️ **Interactive dependency graph** — visualize how failures propagate through the network
- 🎨 **Gen-X cyberpunk design** — dark, gritty, neon accents, scanline effects
- ⚡ **Graceful degradation** — works with mock data when API is offline (perfect for static Vercel deploy)
- 🔥 **Built for judges** — compelling landing page + polished dashboard

## Tech Stack

- **React 19** + TypeScript
- **Vite** — lightning-fast dev & build
- **Tailwind CSS** — utility-first styling
- **Framer Motion** — smooth animations
- **React Flow** — interactive graph visualization
- **Recharts** — data viz (if needed for extensions)
- **Lucide React** — crisp icon library

## Quick Start

```bash
# Install dependencies
npm install --legacy-peer-deps

# Run dev server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Environment Variables

Create `.env` from `.env.example`:

```bash
VITE_API_URL=http://localhost:8080  # Optional, falls back to mock data
```

## Deploy to Vercel

1. Push to GitHub
2. Connect repo to Vercel
3. Vercel auto-detects `vercel.json` config
4. Deploy ✅

The frontend is **fully static** and includes mock data, so it works perfectly on Vercel even without a live backend.

## API Integration

When the backend is running (`uvicorn tina_x.api:app --port 8080`), the frontend will:
- ✅ Fetch real scenarios from `/api/scenarios`
- ✅ Simulate cascades via `/api/simulate`
- ✅ Display live infrastructure graph from `/api/graph`
- ✅ Poll alerts from `/api/alerts`

When the backend is offline, the frontend automatically falls back to mock data and displays a subtle warning badge.

## Design Philosophy

**Gen-X Cyberpunk Noir** — inspired by The Matrix, Hackers, and 90s terminal aesthetics:

- Dark background (`#0a0e14`)
- Neon accents (cyan, magenta, green)
- Monospace fonts (JetBrains Mono)
- Scanline overlays
- Glitch/flicker animations
- Brutalist UI panels

Perfect for a hackathon demo that *looks* as technically impressive as the underlying MeTTa symbolic reasoning.

## License

Part of the **Project-Nzi** ecosystem. See root LICENSE.
