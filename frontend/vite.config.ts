/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Vite config for the Nzi mission-control app.
// - The React plugin enables JSX/Fast Refresh.
// - The `test` block configures Vitest (jsdom so we can render React in tests).
export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
  },
});
