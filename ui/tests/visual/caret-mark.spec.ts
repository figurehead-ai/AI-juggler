/**
 * rc24/rc25 walks K1: after a session's page, every page showed macOS's
 * ⊖ (the input-source mark) at the bottom-left of the content area,
 * where the terminal's caret had been. In WebKit a click on a header tab
 * takes focus from the terminal and leaves no selection at all, and
 * WKWebView then never hides the mark; a caret left somewhere that isn't
 * editable hides it (lib/caret-leaves says how that was measured). In
 * WebKit, as the app runs: Chromium keeps a caret where the field was by
 * itself, so it can't tell.
 */
import { expect, test, type Page } from "@playwright/test";

test.use({ browserName: "webkit", launchOptions: {} });

async function openSession(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await page.getByTestId("rail").locator('[data-instance-id="sess-live-1"]').click({ position: { x: 48, y: 10 } });
  await expect(page.getByTestId("page-tab-terminal")).toHaveAttribute("aria-selected", "true", { timeout: 10_000 });
}

/** Where focus and the caret are: the ⊖ stays while there's no caret. */
const caretState = (page: Page) =>
  page.evaluate(() => {
    const active = document.activeElement as HTMLElement | null;
    const selection = document.getSelection();
    const at = selection?.anchorNode ?? null;
    const host = at instanceof Element ? at : (at?.parentElement ?? null);
    return {
      focus: active?.tagName ?? null,
      fieldFocused: active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement || !!active?.isContentEditable,
      type: selection?.type ?? "None",
      editable: !!host && (host.isContentEditable || !!host.closest("input, textarea")),
    };
  });

test("leaving a terminal's page by a header tab takes the caret, and the ⊖, with it", async ({ page }) => {
  await openSession(page);
  await page.locator(".xterm-helper-textarea").first().focus();
  await page.getByTestId("nav-projects").click();
  await expect(page.locator(".xterm-helper-textarea")).toHaveCount(0);
  const caret = await caretState(page);
  expect(caret.fieldFocused, `focus left on ${caret.focus}`).toBe(false);
  // "None" is the ⊖ left behind; a caret off any field is the ⊖ gone.
  expect(caret, "the caret after leaving the terminal").toMatchObject({ type: "Caret", editable: false });
});

test("a field left for a header tab takes the caret, and the ⊖, with it", async ({ page }) => {
  await openSession(page);
  // Any field, not only the terminal's: the rail's search.
  const search = page.getByRole("searchbox").or(page.getByPlaceholder(/Search/)).first();
  await search.click();
  await page.getByTestId("nav-pools").click();
  expect(await caretState(page)).toMatchObject({ fieldFocused: false, type: "Caret", editable: false });
});
