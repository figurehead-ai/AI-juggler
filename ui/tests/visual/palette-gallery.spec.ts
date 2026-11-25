/**
 * AJ-E2901 gallery: ⌘K as a dialog — before typing (Recent and the top
 * actions, then what's open), typed (groups in order, the matched letters
 * bold, "n more"), the preview beside the list, no matches (Search History
 * active), and offline. AJ-E3824: Suggested and Watch a session… before
 * typing; a Suggested entry's records by kind (sessions, tasks, pools,
 * usage). Both themes, 1280×800 and 1024×700; axe 0
 * critical/serious on each.
 */
import { gate, test, expect } from "./axe-gallery";
import { at, family, open, shot, WIDE_SIZES, THEMES } from "./gallery-04";

const OUT = family("99-palette");
const RECENTS = JSON.stringify([
  { kind: "session", id: "sess-live-1" },
  { kind: "project", id: "proj-payments" },
  { kind: "command", id: "go:usage" },
]);

// AJ-E3824: what the records hold for "What needs me" (generic names).
const row = { name_hidden: false, at_ms: null, started_ms: null, provider: null, app_id: null, app_label: null, desktop: false, project: null, project_id: null, pool: null, pool_id: null, holder: null, files_changed: null, branch: null, used_percent: null, window: null, resets_at_ms: null, blocked_by: null, quote: null, then_state: null, gone: false };
const SEARCH = {
  "what needs me": {
    sessions: [{ ...row, kind: "session", id: "rec-web", title: "web", state: "asks_you", provider: "claude-code", app_label: "Team", project: "shop" }],
    tasks: [{ ...row, kind: "task", id: "rec-export", title: "Wire the export button", state: "blocked", pool: "Release", pool_id: "pool-release", holder: "builder-2" }],
    pools: [{ id: "pool-release", name: "Release", tasks: 1 }],
    usage: [{ ...row, kind: "account", id: "rec-team", title: "Team", state: null, used_percent: 54, window: "weekly" }],
  },
};

for (const theme of THEMES) {
  for (const size of WIDE_SIZES) {
    const where = `${at(size)}-${theme}`;

    test(`Palette — ${where}`, at(size) === "1280x800" ? gate("99-palette") : {}, async ({ page }) => {
      await open(page, theme, size, { __AIJ_SEARCH__: SEARCH }, { "aij.palette.recents": RECENTS });
      await page.keyboard.press("Meta+K");
      const dialog = page.getByRole("dialog", { name: "Search, act or go" });
      await expect(dialog).toBeVisible();
      await expect(page.getByTestId("goto-group-recent")).toBeVisible();
      await shot(page, OUT, `empty-${where}`);

      await page.keyboard.type("pay");
      await expect(page.getByTestId("goto-group-projects")).toBeVisible();
      await shot(page, OUT, `typed-${where}`);

      await page.getByRole("combobox").fill("");
      await page.getByTestId("goto-group-suggested").getByText("What needs me").click();
      await expect(page.getByTestId("goto-group-tasks")).toBeVisible();
      await shot(page, OUT, `records-${where}`);

      // Preview the first session.
      const field = page.getByRole("combobox");
      await field.fill("wrk");
      const sessions = page.getByTestId("goto-group-sessions");
      await expect(sessions).toBeVisible();
      const first = await page.locator("[role='option']").evaluateAll((rows) => rows.findIndex((row) => row.closest("[data-testid='goto-group-sessions']")));
      for (let n = 0; n < first; n++) await page.keyboard.press("ArrowDown");
      await page.keyboard.press("ArrowRight");
      await expect(page.getByTestId("goto-preview")).toBeVisible();
      await shot(page, OUT, `preview-${where}`);

      await page.keyboard.press("ArrowLeft");
      await field.fill("qqqzz");
      await expect(page.getByTestId("goto-no-matches")).toBeVisible();
      await shot(page, OUT, `no-matches-${where}`);
    });

    test(`Palette offline — ${where}`, async ({ page }) => {
      await open(page, theme, size, { __AIJ_OFFLINE__: { since_ms: 1_728_799_000_000 } });
      await page.keyboard.press("Meta+K");
      await expect(page.getByTestId("goto-offline")).toBeVisible();
      await shot(page, OUT, `offline-${where}`);
    });
  }
}
