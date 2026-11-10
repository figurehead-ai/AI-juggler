/**
 * AJ-E3306 (the rc26 persona walk): New session in an 800×600 window cut
 * its meaning with an ellipsis: the folder line ("worked here just n…").
 * At a narrow width it wraps; nothing in the sheet is cut, and no control
 * covers its text. The memory note was never cut, but "Tight — this Mac
 * can take it, just" read as if it were; it says "can just take it".
 */
import { expect, test, type Page } from "@playwright/test";

const T0 = new Date(2026, 10, 2, 15, 0, 0).getTime();
const account = (id: string, provider: string, label: string) => ({ id, provider, label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [account("claude-work", "claude-code", "Work Max")];
const RECIPES = [{ id: "p-1", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null }];
// A long project path, worked on a moment ago.
const PROJECTS = [
  { id: "proj-web", root: "/Users/you/Dev/clients/northwind/web-storefront-checkout", name: "web-storefront-checkout", last_activity_ms: T0 - 20_000, active_instance: null, accounts_used: ["Work Max"], instance_count: 3, history_sessions: 2, history_last_used_ms: null, repo: null, missing: false, pinned: false },
];
const ADVICE = [
  {
    recipe_id: "p-1",
    verdict: "tight",
    words: "Tight — this Mac can just take it",
    reasons: ["Memory is 88% used", "the Claude app holds 3.1 GB"],
    projected_bytes: 2_100_000_000,
    based_on: 6,
    relief_session_id: null,
    relief_bytes: null,
  },
];

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, recipes, advice, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      Object.assign(w, { __AIJ_FIXTURE__: fixture, __AIJ_RECIPES__: recipes, __AIJ_RANKING__: [], __AIJ_ADVICE__: advice });
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: ACCOUNTS, projects: PROJECTS, sessions: [] }, recipes: RECIPES, advice: ADVICE, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
  await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
  await expect(page.getByTestId("new-session-sheet")).toBeVisible();
  await page.getByTestId("change-folder").click();
  await page.getByRole("menu", { name: "Where to work" }).getByText("web-storefront-checkout", { exact: true }).click();
  await expect(page.getByTestId("new-session-sentence")).toContainText("Start in web-storefront-checkout");
  await expect(page.getByTestId("machine-advice")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  test(`New session at 800×600 wraps its words, never cuts them or covers them — ${theme}`, async ({ page }) => {
    await open(page, theme);
    const sheet = page.getByTestId("launch-preselected");
    const found = await sheet.evaluate((root) => {
      const cut: string[] = [];
      const covered: string[] = [];
      const words = (el: Element) => (el.textContent ?? "").trim().slice(0, 60);
      for (const el of root.querySelectorAll<HTMLElement>("*")) {
        const style = getComputedStyle(el);
        if (style.textOverflow === "ellipsis" && el.scrollWidth > el.clientWidth + 1) cut.push(words(el));
      }
      // Every text run against every control that isn't its own.
      const controls = [...root.querySelectorAll<HTMLElement>("button, [role='button'], a[href]")];
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        if (!(node.textContent ?? "").trim()) continue;
        const range = document.createRange();
        range.selectNodeContents(node);
        for (const box of range.getClientRects()) {
          for (const control of controls) {
            if (control.contains(node)) continue;
            const c = control.getBoundingClientRect();
            const overlap = Math.min(box.right, c.right) - Math.max(box.left, c.left) > 1 && Math.min(box.bottom, c.bottom) - Math.max(box.top, c.top) > 1;
            if (overlap) covered.push(`${(node.textContent ?? "").trim().slice(0, 40)} under ${words(control)}`);
          }
        }
      }
      return { cut, covered };
    });
    expect(found.cut, "text cut with an ellipsis").toEqual([]);
    expect(found.covered, "a control over text").toEqual([]);
    // The two lines the walk saw cut, whole.
    await expect(sheet).toContainText("worked here just now");
    await expect(page.getByTestId("machine-advice")).toContainText("Tight — this Mac can just take it");
  });
}
