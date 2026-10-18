import { defineConfig } from "@playwright/test";

// AJ-E802: visual-states harness. Runs against the PRODUCTION build
// (vite preview) for determinism — never a live Tauri backend.
export default defineConfig({
  testDir: "./tests/visual",
  timeout: 30_000,
  retries: 0,
  use: {
    baseURL: "http://localhost:4173",
  },
  webServer: {
    command: "pnpm build && pnpm exec vite preview --port 4173 --strictPort",
    url: "http://localhost:4173",
    reuseExistingServer: false,
    timeout: 120_000,
  },
  reporter: [["list"]],
});
