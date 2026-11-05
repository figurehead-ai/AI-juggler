/**
 * AJ-E2906 gallery: first run's found step saying how sure it is — Found,
 * Likely and Guessed rows with where each came from, a sign-in that may
 * have expired, a row's Details open, and Look again. Both themes,
 * 1280×800 and 1024×700; axe 0 critical/serious on each.
 */
import { gate, test, expect } from "./axe-gallery";
import { goTo } from "./fixtures";
import { at, family, open, shot, SIZES, THEMES } from "./gallery-04";

const OUT = family("99-first-run-confidence");
const NOW_S = Math.floor(1_728_800_000_000 / 1000);
const row = (path: string, origin: string, inference: Record<string, unknown>, extra: Record<string, unknown> = {}) => ({
  harness: "claude-code", path, origin, set_by: null, login: "logged_in", bytes: 4_000_000, last_used_secs: NOW_S - 86_400, managed: false, in_use: [], signals: [origin],
  notes: ["A separate Claude configuration — created manually or by another tool."],
  inference: { purpose: "Main", purpose_confidence: "found", purpose_rule: "usual-folder", login: "signed-in", login_confidence: "found", login_rule: "sign-in-present", activity: "active", activity_confidence: "found", activity_rule: "newest-conversation", activity_words: "Active this week", keychain_bound: false, ...inference },
  ...extra,
});
const SCAN = {
  installs: [
    row("/Users/you/.claude", "vendor-default", {}, { notes: ["Claude Code's standard location — what a plain `claude` command uses."] }),
    row("/Users/you/.claude-work", "home-pattern", { purpose: "Work", purpose_confidence: "likely", purpose_rule: "name-word" }),
    row("/Users/you/.claude-x7", "env-override", { purpose: "X7", purpose_confidence: "guessed", purpose_rule: "folder-name", login: "not-signed-in", login_rule: "no-sign-in" }, { login: "logged_out", last_used_secs: NOW_S - 50 * 86_400 }),
    row("/Users/you/.claude-old", "other-manager", { purpose: "Old", purpose_confidence: "guessed", purpose_rule: "folder-name", login: "maybe-expired", login_confidence: "likely", login_rule: "sign-in-empty" }, { login: "logged_out", set_by: "cc-switch", last_used_secs: NOW_S - 200 * 86_400 }),
  ],
  probes: [{ label: "Folders named .claude…", health: "ok", detail: "4 setups", reason: null }],
  seeds: [],
  desktop_apps: [],
  cancelled: false,
};

for (const theme of THEMES) {
  for (const size of SIZES) {
    const where = `${at(size)}-${theme}`;
    test(`First run's confidence — ${where}`, at(size) === "1280x800" ? gate("99-first-run-confidence") : {}, async ({ page }) => {
      await open(page, theme, size, { __AIJ_SCAN_REPORT__: SCAN });
      await goTo(page, "Accounts");
      await page.getByRole("button", { name: "Add an app" }).first().click();
      await page.getByRole("dialog", { name: /Add an app/ }).getByRole("button", { name: "Claude Code →" }).click();
      const found = page.getByTestId("first-run-found");
      await expect(found.getByTestId("first-run-confidence").first()).toBeVisible();
      await shot(page, OUT, `found-${where}`);
      await page.getByRole("button", { name: "Details for claude-old" }).click();
      await expect(page.getByTestId("first-run-details")).toBeVisible();
      await shot(page, OUT, `details-${where}`);
    });
  }
}
