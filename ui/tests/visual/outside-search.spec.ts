/**
 * AJ-E1538 gallery: Search the words of every session. Settings › Privacy
 * off (the size its index would take), on (the index's size as it
 * builds), and History finding a session started outside AI Juggler by
 * what was said in it. Both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, settingsPane } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/90-outside-search");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light", enabled: boolean) {
  await page.addInitScript(
    ([theme, enabled]) => {
      localStorage.setItem("aij.theme", theme as string);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_OUTSIDE_SEARCH__ = enabled
        ? { enabled: true, size_bytes: 212_000_000, sessions: 3412, indexed: 1180 }
        : { enabled: false, size_bytes: null, sessions: 3412, indexed: 0 };
      w.__AIJ_OUTSIDE_SEARCH_ESTIMATE__ = { size_bytes: 684_000_000, sessions: 3412 };
      w.__AIJ_TRANSCRIPT_HITS__ = [
        {
          session_id: null,
          account_id: "claude-personal",
          conversation_id: "outside-2",
          snippet: "…add cursor pagination to the orders endpoint and keep the old page param working…",
        },
      ];
    },
    [theme, enabled] as const,
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  for (const enabled of [false, true]) {
    test(`Settings › Privacy, search every session ${enabled ? "on" : "off"} — ${theme}`, async ({ page }) => {
      await open(page, theme, enabled);
      await goTo(page, "Settings");
      await settingsPane(page, "Privacy");
      const setting = page.getByTestId("privacy-outside-search");
      await expect(page.getByTestId("outside-search-size")).toHaveText(
        enabled
          ? "Index: 212 MB so far · 1180 of 3412 sessions. It builds in the background."
          : "Its index would take about 684 MB on this computer, for 3412 sessions.",
      );
      await setting.scrollIntoViewIfNeeded();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `settings-${enabled ? "on" : "off"}-1280x800-${theme}.png`) });
      await axe(page, `outside search ${enabled ? "on" : "off"} ${theme}`);
    });
  }

  test(`History finds an outside session by its words — ${theme}`, async ({ page }) => {
    await open(page, theme, true);
    await goTo(page, "History");
    await page.getByTestId("history-search").fill("pagination");
    await expect(page.getByTestId("history-snippet").first()).toContainText("cursor pagination");
    await expect(page.getByTestId("history-scope")).toContainText("Words are searched in every session.");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `history-1280x800-${theme}.png`) });
    await axe(page, `history outside hit ${theme}`);
  });
}
