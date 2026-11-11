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

async function openSession(page: Page, id = "sess-live-1") {
  await page.addInitScript(() => {
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await page.locator(`[data-instance-id="${id}"]`).first().click({ position: { x: 48, y: 10 } });
  await expect(page.locator(".xterm-helper-textarea")).toHaveCount(1, { timeout: 10_000 });
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

/*
 * rc26 walk K1: the ⊖ stayed below a stopped session's terminal, over
 * "ran … · scrollback replayed", and sharp over a dialog's scrim. macOS
 * moves the mark only with the caret: a frame that moves while its field
 * keeps focus leaves the mark where the caret was (WKWebView harness:
 * the mark comes back at the new place only when the field is focused
 * again ~120 ms or more after it lost focus).
 */

test("an ended session's terminal holds no focus, nor takes it", async ({ page }) => {
  await openSession(page, "sess-done-1");
  // The mount's own first focus has had its frame.
  await page.waitForTimeout(400);
  const field = page.locator(".xterm-helper-textarea");
  expect((await caretState(page)).fieldFocused, "an ended terminal took focus on open").toBe(false);
  await field.evaluate((element) => (element as HTMLTextAreaElement).focus());
  expect((await caretState(page)).fieldFocused, "an ended terminal's field can be focused").toBe(false);
});

test("a terminal whose frame moves gives focus up and takes it back, so the ⊖ follows its caret", async ({ page }) => {
  await openSession(page);
  const field = page.locator(".xterm-helper-textarea");
  await field.evaluate((element) => {
    (element as HTMLTextAreaElement).focus();
    const seen = ((window as unknown as { __focusTrail: string[] }).__focusTrail = []);
    element.addEventListener("blur", () => seen.push("blur"));
    element.addEventListener("focus", () => seen.push("focus"));
  });
  // What an ended session's page does: a line below the frame shrinks it.
  await page.getByTestId("terminal").evaluate((terminal) => {
    const line = document.createElement("p");
    line.textContent = "ran 2m · scrollback replayed";
    line.style.cssText = "height:21px;margin:0;flex:none";
    terminal.closest(".min-h-0.flex-1.flex-col, .flex-1")!.parentElement!.append(line);
  });
  await expect.poll(() => page.evaluate(() => (window as unknown as { __focusTrail: string[] }).__focusTrail.join(" ")), { timeout: 3000 }).toBe("blur focus");
  expect((await caretState(page)).fieldFocused, "the terminal has focus again").toBe(true);
});

test("a dialog opened from a focused terminal takes focus from it", async ({ page }) => {
  await openSession(page);
  await page.locator(".xterm-helper-textarea").focus();
  // ⌘⇧N: New project, whose sheet sets no first focus of its own.
  await page.keyboard.press("Meta+Shift+N");
  const dialog = page.locator('[aria-modal="true"]');
  await expect(dialog).toHaveCount(1);
  await expect.poll(() => page.evaluate(() => !!document.activeElement?.closest('[aria-modal="true"]')), { timeout: 2000 }).toBe(true);
  expect(await caretState(page), "the caret behind the dialog").toMatchObject({ fieldFocused: false, type: "Caret", editable: false });
});

test("keys typed while a moved terminal lets focus go all reach it, in order", async ({ page }) => {
  await openSession(page);
  await page.evaluate(() => {
    const seams = window as unknown as { __AIJ_TERMINALS__: Record<string, { term: { onData: (on: (data: string) => void) => void } }>; __typed: string[] };
    seams.__typed = [];
    seams.__AIJ_TERMINALS__["sess-live-1"]!.term.onData((data) => seams.__typed.push(data));
    document.querySelector<HTMLTextAreaElement>(".xterm-helper-textarea")!.focus();
  });
  await page.getByTestId("terminal").evaluate((terminal) => {
    const line = document.createElement("p");
    line.textContent = "ran 2m · scrollback replayed";
    line.style.cssText = "height:21px;margin:0;flex:none";
    terminal.closest(".min-h-0.flex-1.flex-col, .flex-1")!.parentElement!.append(line);
  });
  // The gap: focus is on nothing until the terminal takes it back.
  await expect.poll(() => page.evaluate(() => document.activeElement === document.body), { timeout: 2000 }).toBe(true);
  await page.keyboard.type("ls -la");
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("Enter");
  await expect.poll(() => page.evaluate(() => document.activeElement?.classList.contains("xterm-helper-textarea")), { timeout: 2000 }).toBe(true);
  await page.keyboard.type("ok");
  const typed = await page.evaluate(() => (window as unknown as { __typed: string[] }).__typed.join(""));
  expect(typed, "what the terminal got").toBe("ls -la\u001b[A\rok");
});

test("an ended session's page keeps the terminal's ⌘F and ⌘C", async ({ page }) => {
  await openSession(page, "sess-done-1");
  await page.evaluate(() => {
    const seams = window as unknown as { __copied: string[] };
    seams.__copied = [];
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: (text: string) => (seams.__copied.push(text), Promise.resolve()) } });
  });
  await page.waitForTimeout(400);
  await page.keyboard.press("Meta+c");
  await expect.poll(() => page.evaluate(() => (window as unknown as { __copied: string[] }).__copied.length)).toBe(1);
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("terminal-search-input")).toBeVisible();
});
