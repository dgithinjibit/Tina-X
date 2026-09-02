/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Gen-X cyberpunk palette: dark, gritty, neon accents
        'cyber': {
          'dark': '#0a0e14',
          'darker': '#050709',
          'panel': '#151b23',
          'border': '#1f2830',
          'gray': '#3d4752',
          'text': '#c9d1d9',
          'muted': '#8b949e',
        },
        'neon': {
          'green': '#39ff14',
          'cyan': '#00ffff',
          'magenta': '#ff00ff',
          'yellow': '#ffff00',
          'red': '#ff1744',
        },
        'status': {
          'healthy': '#10b981',
          'warning': '#f59e0b',
          'critical': '#ef4444',
          'offline': '#6b7280',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Courier New', 'monospace'],
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
      animation: {
        'glow': 'glow 2s ease-in-out infinite',
        'scan': 'scan 4s linear infinite',
        'flicker': 'flicker 0.15s infinite',
      },
      keyframes: {
        glow: {
          '0%, 100%': { textShadow: '0 0 8px currentColor' },
          '50%': { textShadow: '0 0 16px currentColor' },
        },
        scan: {
          '0%': { transform: 'translateY(-100%)' },
          '100%': { transform: 'translateY(100%)' },
        },
        flicker: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.8' },
        },
      },
    },
  },
  plugins: [],
}
