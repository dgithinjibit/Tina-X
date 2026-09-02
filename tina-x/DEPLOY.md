# TINA-X Deployment Guide

## Quick Start (Local Development)

### Backend
```bash
cd tina-x
python -m venv .venv
source .venv/bin/activate  # or `.venv\Scripts\activate` on Windows
pip install -e .
pip install fastapi uvicorn

# Run API server
uvicorn tina_x.api:app --reload --port 8080

# Test
curl http://localhost:8080/api/health
```

### Frontend
```bash
cd tina-x/frontend
npm install
npm run dev

# Open http://localhost:5173
```

## Deploy Frontend to Vercel

### Option 1: Vercel CLI
```bash
cd tina-x/frontend
npm install -g vercel
vercel --prod
```

### Option 2: Vercel Dashboard
1. Go to https://vercel.com/new
2. Import your GitHub repo: `dgithinjibit/Tina-X` (or `Project-Nzi`)
3. Set **Root Directory**: `tina-x/frontend`
4. Framework Preset: **Vite**
5. Build Command: `npm run build`
6. Output Directory: `dist`
7. Install Command: `npm install --legacy-peer-deps`
8. Deploy ✅

### Environment Variables (Optional)
If you have a live backend deployed, add:
```
VITE_API_URL=https://your-backend-url.com
```

Otherwise, the frontend uses mock data automatically.

## Deploy Backend (Optional)

The frontend works standalone with mock data, but if you want the full live experience:

### Railway / Render / Fly.io
```bash
# Add to tina-x/requirements.txt
fastapi
uvicorn[standard]

# Procfile or start command:
web: uvicorn tina_x.api:app --host 0.0.0.0 --port $PORT
```

### AWS Lambda + API Gateway
Use Mangum adapter:
```python
from mangum import Mangum
from tina_x.api import app

handler = Mangum(app)
```

## Hackathon Demo Flow

1. **Landing Page** — Gen-X cyberpunk aesthetic, feature grid, CTA
2. **Click "Launch Dashboard"** → Main control center
3. **Select a hazard scenario** (e.g., "Compound: Earthquake + Typhoon")
4. **Watch cascade failures propagate** in real-time:
   - CascadeViewer shows failures with details
   - InfraGraph highlights failed nodes with glow effects
   - AlertPanel shows live alerts (if backend connected)
5. **Explain symbolic AI advantage** — MeTTa reasons about out-of-distribution events deep learning can't handle

## Architecture

```
┌─────────────────┐
│  React Frontend │  (Static, deployed on Vercel)
│  Gen-X Theme    │  - Landing + Dashboard
│  Port 5173      │  - Mock data fallback
└────────┬────────┘
         │ HTTP/JSON
         │ (optional)
┌────────▼────────┐
│  FastAPI Backend│  (Python, optional deployment)
│  Port 8080      │  - /api/scenarios
└────────┬────────┘  - /api/simulate
         │           - /api/graph
┌────────▼────────┐  - /api/alerts
│  MeTTa Engine   │
│  Symbolic AI    │  - cascade.metta
│  Reasoner       │  - graph.metta
└─────────────────┘  - TinaEngine (Python bridge)
```

## Tech Stack

**Frontend**
- React 18 + TypeScript
- Vite (dev/build)
- Tailwind CSS (cyberpunk design)
- Framer Motion (animations)
- React Flow (graph viz)
- React Router (SPA routing)

**Backend**
- FastAPI (REST API)
- MeTTa (symbolic reasoning)
- Python 3.12+

**Design Philosophy**
- **Gen-X Aesthetic**: Dark, gritty, neon accents, scanlines
- **Hackathon-ready**: Compelling visuals + explainable AI
- **Resilient**: Works offline with mock data
- **Fast**: Static frontend, sub-second builds

## Repository

GitHub: https://github.com/dgithinjibit/Tina-X
(or https://github.com/dgithinjibit/Project-Nzi → `tina-x/` subdirectory)

## License

Part of the **Project-Nzi** ecosystem — two-rate brain architecture for disaster response.
