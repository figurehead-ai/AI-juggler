/**
 * The rc31 NY1 walk: on the archive and unarchive toasts, "Undo (3s)" sat
 * over the words ("…archived — nothing deleted."). A control never
 * covers text: Undo goes under the words, and the sentence after a name
 * wraps. Checked in a narrow window (800 × 600) and a small one.
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses } from "./fixtures";

const T0 = new Date(2026, 10, 2, 15, 0, 0).getTime();

async function open(page: Page, theme: "dark" | "light", size: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      Object.assign(w, { __AIJ_FIXTURE__: fixture, __AIJ_TOAST_PAUSED__: true });
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(size);
  await page.goto("/");
  await page.locator("header").getByRole("button", { name: "History", exact: true }).click();
}

/** Each text run in `root` that a control (not its own) covers. */
async function covered(page: Page, testId: string): Promise<string[]> {
  return page.getByTestId(testId).evaluate((root) => {
    const out: string[] = [];
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
          if (overlap) out.push(`${(node.textContent ?? "").trim().slice(0, 40)} under ${(control.textContent ?? "").trim()}`);
        }
      }
    }
    // and nothing runs out of the toast
    const edge = root.getBoundingClientRect();
    for (const el of root.querySelectorAll<HTMLElement>("p, span")) {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && (r.right > edge.right + 1 || r.left < edge.left - 1)) out.push(`${(el.textContent ?? "").trim().slice(0, 40)} out of the toast`);
    }
    return out;
  });
}

for (const theme of ["dark", "light"] as const) {
  for (const size of [
    { width: 800, height: 600 },
    { width: 1024, height: 680 },
  ]) {
    test(`the archive and unarchive toasts keep Undo off their words — ${size.width}×${size.height} — ${theme}`, async ({ page }) => {
      await open(page, theme, size);
      const row = page.getByTestId("page-session-item").first();
      await expect(row).toBeVisible();
      await row.click({ button: "right" });
      await page.getByRole("menuitem", { name: /^Archive/ }).click();
      const toast = page.getByTestId("toast");
      await expect(toast).toContainText("archived — nothing deleted.");
      await expect(toast.getByRole("button", { name: /Undo/ })).toBeVisible();
      expect(await covered(page, "toast")).toEqual([]);
      await page.getByTestId("history-tab-archived").click();
      const archived = page.getByTestId("page-session-item").first();
      await archived.click({ button: "right" });
      await page.getByRole("menuitem", { name: /^Unarchive/ }).click();
      await expect(toast).toContainText("unarchived.");
      expect(await covered(page, "toast")).toEqual([]);
    });
  }
}
