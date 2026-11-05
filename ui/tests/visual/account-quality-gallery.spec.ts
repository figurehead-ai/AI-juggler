/**
 * AJ-E2903 gallery: an Account's "How its sessions went" (the last 7
 * days, each signal with its source), and the same for a tool that
 * records neither errors nor context. Both themes, 1280×800 and
 * 1024×700; axe 0 critical/serious on each.
 */
import { gate, test, expect } from "./axe-gallery";
import { at, family, open, shot, SIZES, THEMES } from "./gallery-04";

const OUT = family("99-account-quality");
const FULL = { tool: "claude-code", sessions: 14, pending: 1, errors: 9, turns: 300, errors_measured: true, stuck_episodes: 2, context_avg_pct: 71.4, context_sessions: 11, finished: 9, tokens_per_finished: 412_000 };
const BARE = { tool: "codex", sessions: 3, pending: 0, errors: 0, turns: 0, errors_measured: false, stuck_episodes: 0, context_avg_pct: null, context_sessions: 0, finished: 0, tokens_per_finished: null };

for (const theme of THEMES) {
  for (const size of SIZES) {
    const where = `${at(size)}-${theme}`;
    for (const [name, info] of [["measured", FULL], ["not-reported", BARE]] as const) {
      test(`How its sessions went, ${name} — ${where}`, at(size) === "1280x800" && name === "measured" ? gate("99-account-quality") : {}, async ({ page }) => {
        await open(page, theme, size, { __AIJ_ACCOUNT_QUALITY__: info });
        await page.getByRole("link", { name: "Accounts" }).or(page.getByRole("button", { name: "Accounts", exact: true })).first().click();
        await page.getByTestId("account-page").first().click();
        const section = page.getByTestId("account-quality");
        await expect(section).toBeVisible();
        await section.scrollIntoViewIfNeeded();
        await shot(page, OUT, `${name}-${where}`);
      });
    }
  }
}
