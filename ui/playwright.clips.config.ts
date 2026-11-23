import { defineConfig } from "@playwright/test";

// AJ-E1546: records the Welcome's, How it works' and the permission
// cards' clips from the gallery's generic fixtures (tests/clips). Run by
// hand (scripts/record_clips.sh), never by a gate: it writes public/clips.
// Playwright's own cached Chromium and ffmpeg only; nothing is fetched.
const PORT = Number(process.env.AIJ_PW_PORT ?? 4183);

export default defineConfig({
  testDir: "./tests/clips",
  timeout: 120_000,
  retries: 0,
  workers: 1,
  use: {
    baseURL: `http://localhost:${PORT}`,
    launchOptions: { args: ["--disable-3d-apis"] },
  },
  webServer: {
    // The type check is the gates' job; a recording needs only the bundle.
    command: `pnpm exec vite build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    timeout: 180_000,
  },
  reporter: [["list"]],
});
