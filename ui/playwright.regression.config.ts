import { defineConfig } from "@playwright/test";

// Another checkout's suite may hold 4173 (see playwright.config.ts).
const PORT = Number(process.env.AIJ_PW_PORT ?? 4173);

// AJ-E803: the visual regression gate. Baselines are rendered in the
// pinned Playwright container (scripts/visual_gate.sh), so they are
// compared only there — never against a developer's own screen.
export default defineConfig({
  testDir: "./tests/visual",
  testMatch: /regression\.spec\.ts/,
  snapshotPathTemplate: "{testDir}/__baselines__/{platform}/{arg}{ext}",
  timeout: 30_000,
  retries: 0,
  workers: 4,
  use: { baseURL: `http://localhost:${PORT}` },
  webServer: {
    command: `pnpm build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
  reporter: [["list"], ["json", { outputFile: "test-results/visual-gate.json" }]],
});
