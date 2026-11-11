import { defineConfig } from "happo";

// AJ-E2020 R6 (D-99 point 6, Happo in place of Argos): visual review on
// Happo, free for open source. CI signs in with GitHub's OIDC token
// (.github/workflows/happo.yml): no key or secret is stored anywhere, and
// these stay unset there. A key pair, if one is ever used, comes from the
// environment only. Happo runs only under `happo -- playwright test` in CI;
// never locally (each run is a report on the account).
export default defineConfig({
  apiKey: process.env.HAPPO_API_KEY,
  apiSecret: process.env.HAPPO_API_SECRET,
  integration: { type: "playwright" },
  targets: {
    "chrome-large": { type: "chrome", viewport: "1200x900" },
    "chrome-small": { type: "chrome", viewport: "375x667" },
    accessibility: { type: "accessibility", viewport: "1024x768" },
  },
});
