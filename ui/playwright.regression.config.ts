import { defineConfig } from "@playwright/test";

// Another checkout's suite may hold 4173 (see playwright.config.ts).
const PORT = Number(process.env.AIJ_PW_PORT ?? 4173);

/** AJ-E1522: the gallery specs whose families have a gate state. Only
 * these load in the gate (others read files outside `ui/`, which the
 * container doesn't mount). */
const GALLERY_SPECS = [
  "account-manage-gallery", "account-settings-gallery", "accounts-gallery", "budget-caps",
  "burn-down", "capture", "diagnostics", "fleet-launch", "home-gallery", "journey-first-run",
  "keep-going", "launch-advisor-gallery", "limit-ready", "marks-gallery", "menus-gallery",
  "notification-centre-gallery", "notifications", "open-pack", "outside-gallery", "overload-gallery", "pages-gallery",
  "privacy-gallery", "processes-gallery", "project-manage-gallery", "project-sync-gallery", "rail-narrow",
  "switch-setup-gallery",
  "rail-projects", "terminal-tools", "this-mac-gallery", "this-mac-page",
  // 0.2's own families: pools, agent tools, messages, tasks, more tools,
  // service accounts, and the Usage dashboard.
  "agent-tools-gallery", "messages-gallery", "more-tools-gallery", "pools-gallery",
  "service-accounts-gallery", "tasks-gallery", "usage-gallery",
];

// AJ-E803: the visual regression gate. Baselines are rendered in the
// pinned Playwright container (scripts/visual_gate.sh), so they are
// compared only there — never against a developer's own screen.
export default defineConfig({
  testDir: "./tests/visual",
  projects: [
    { name: "regression", testMatch: /regression\.spec\.ts/ },
    // AJ-E1522: one state per evidence-gallery family (tagged with
    // `gate()` in its spec), compared like the screens above.
    {
      name: "gallery",
      testMatch: new RegExp(`(^|/)(${GALLERY_SPECS.join("|")})\\.spec\\.ts$`),
      grep: /@gate/,
      // A family's flow is longer than one screen's open (axe included).
      timeout: 120_000,
    },
  ],
  snapshotPathTemplate: "{testDir}/__baselines__/{platform}/{arg}{ext}",
  timeout: 30_000,
  retries: 0,
  workers: 4,
  use: { baseURL: `http://localhost:${PORT}` },
  webServer: {
    command: `pnpm build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    // The build runs inside the container; under emulation on a busy Mac
    // (an arm64 Mac running the amd64 image) it takes minutes.
    timeout: 600_000,
  },
  reporter: [["list"], ["json", { outputFile: "test-results/visual-gate.json" }]],
});
