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
  },
  webServer: {
    command: `pnpm build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
  reporter: [["list"]],
});
