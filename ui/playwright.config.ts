import { defineConfig } from "@playwright/test";

// Several checkouts on one Mac can run the suite at once. Two previews on
// one port can both start (one on IPv4, one on IPv6) and a run then tests
// the other checkout's build, so each checkout picks its own AIJ_PW_PORT.
const PORT = Number(process.env.AIJ_PW_PORT ?? 4173);

// AJ-E802: visual-states harness. Runs against the PRODUCTION build
// (vite preview) for determinism — never a live Tauri backend.
export default defineConfig({
  testDir: "./tests/visual",
  // AJ-E803's regression gate runs in its pinned container only.
  testIgnore: /regression\.spec\.ts/,
  timeout: 30_000,
  retries: 0,
  use: {
    baseURL: `http://localhost:${PORT}`,
    // 0.3.2: no WebGL, so the terminal keeps its DOM renderer (its
    // fallback): captures stay deterministic and its rows readable. No
    // partial raster: a tile redrawn in part kept a few edge pixels 1 off
    // from one load to the next (the determinism test, 1 run in 4; 0 in
    // 24 pairs without it: the 0.5 gate triage).
    launchOptions: { args: ["--disable-3d-apis", "--disable-partial-raster"] },
  },
  webServer: {
    command: `pnpm build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
  reporter: [["list"]],
});
