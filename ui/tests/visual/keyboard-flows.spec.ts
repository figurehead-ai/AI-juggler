/**
 * AJ-E804 Req.2 — the six core flows, keyboard only: launch, switch
 * (1-click), switch (wizard), cleanup confirm, onboarding, and the
 * inspector tab walk. No mouse after the page loads. Every focus stop
 * on the way must show a visible indicator (an outline or a ring);
 * dialogs trap focus while open and Esc closes them, returning focus to
 * the control that opened them. Each flow's stops are written to the
 * AJ-E804 evidence as its traversal record (under `just a11y-gate`).
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, DOOR_ONECLICK, DOOR_WIZARD, honestEmpty, type FixtureSet } from "./fixtures";

const T0 = 1_728_800_000_000;
const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E804/keyboard");
// The traversal records are written under `just a11y-gate` only; every
// run still asserts them.
const EVIDENCE = process.env.AJ_A11Y_EVIDENCE === "1";

type Stop = { name: string; visible: boolean };

async function open(page: Page, fixture: Partial<FixtureSet> = allStatuses, extra: Record<string, unknown> = {}, onboarded = true) {
  await page.addInitScript(
    ({ fixture, t0, extra, onboarded }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [k, v] of Object.entries(extra)) w[k] = v;
      localStorage.setItem("aij.theme", "dark");
      if (onboarded) {
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      }
      Date.now = () => t0;
    },
    { fixture, t0: T0, extra, onboarded },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

/** The focused element: its accessible-ish name, and whether focus shows. */
async function focused(page: Page): Promise<Stop> {
  return page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (!el || el === document.body) return { name: "(body)", visible: false };
    const style = getComputedStyle(el);
    const outline = style.outlineStyle !== "none" && parseFloat(style.outlineWidth) > 0;
    const ring = style.boxShadow !== "none" && style.boxShadow !== "";
    const labelled = (el as HTMLInputElement).labels?.[0]?.textContent?.replace(/\s+/g, " ").trim().slice(0, 60);
    // Name from content, as accessible names are: no hidden or
    // aria-hidden text (a row's description lives in a hidden node).
    const named = el.cloneNode(true) as HTMLElement;
    named.querySelectorAll("[hidden], [aria-hidden='true']").forEach((node) => node.remove());
    const label =
      el.getAttribute("aria-label") ||
      labelled ||
      (named.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 60) ||
      el.getAttribute("title") ||
      el.getAttribute("data-testid") ||
      (el.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 60) ||
      el.tagName.toLowerCase();
    return { name: `${el.getAttribute("role") ?? el.tagName.toLowerCase()}: ${label}`, visible: outline || ring };
  });
}

/** Tab (or Shift+Tab) until `match` holds for the focused element; every stop is recorded. */
async function tabTo(page: Page, stops: Stop[], match: RegExp, { max = 80, back = false } = {}): Promise<void> {
  for (let i = 0; i < max; i++) {
    await page.keyboard.press(back ? "Shift+Tab" : "Tab");
    const stop = await focused(page);
    stops.push(stop);
    if (match.test(stop.name)) return;
  }
  throw new Error(`never reached ${match} by keyboard; stops:\n${stops.map((s) => s.name).join("\n")}`);
}

/** Tab until the focused element matches `selector`. */
async function tabToEl(page: Page, stops: Stop[], selector: string, { max = 80, back = false } = {}): Promise<void> {
  for (let i = 0; i < max; i++) {
    await page.keyboard.press(back ? "Shift+Tab" : "Tab");
    stops.push(await focused(page));
    if (await page.evaluate((sel) => document.activeElement?.matches(sel) ?? false, selector)) return;
  }
  throw new Error(`never reached ${selector} by keyboard; stops:\n${stops.map((s) => s.name).join("\n")}`);
}

/** ↓ (or ↑) inside a list until `match` holds for the focused element;
 * every stop is recorded (F11: the rail's list is one Tab stop, its rows
 * are reached with the arrows). */
async function arrowTo(page: Page, stops: Stop[], match: RegExp, { max = 60, up = false } = {}): Promise<void> {
  for (let i = 0; i < max; i++) {
    const stop = await focused(page);
    if (i > 0) stops.push(stop);
    if (match.test(stop.name)) return;
    await page.keyboard.press(up ? "ArrowUp" : "ArrowDown");
  }
  throw new Error(`never reached ${match} with the arrows; stops:\n${stops.map((s) => s.name).join("\n")}`);
}

/** Leaves the terminal if a selection put focus there. */
async function leaveTerminalIfFocused(page: Page, stops: Stop[]) {
  // The terminal takes focus on the frame after it mounts.
  await page.waitForTimeout(100);
  const here = await focused(page);
  if (here.name.includes("Terminal input")) {
    stops.push(here);
    await page.keyboard.press("Control+Tab");
    stops.push(await focused(page));
  }
}

function record(flow: string, stops: Stop[], notes: string[] = []) {
  const hidden = stops.filter((s) => !s.visible && s.name !== "(body)");
  if (EVIDENCE) mkdirSync(OUT, { recursive: true });
  if (EVIDENCE) writeFileSync(
    resolve(OUT, `${flow}.md`),
    [
      `# Keyboard traversal — ${flow}`,
      "",
      "Generated by `ui/tests/visual/keyboard-flows.spec.ts` (1280x800, dark).",
      "",
      "| # | Focus stop | Focus visible |",
      "|---|---|---|",
      ...stops.map((s, i) => `| ${i + 1} | ${s.name.replace(/\|/g, "/")} | ${s.visible ? "yes" : "**NO**"} |`),
      "",
      ...notes.map((n) => `- ${n}`),
      "",
    ].join("\n"),
  );
  expect(hidden.map((s) => s.name), `${flow}: focus stops without a visible indicator`).toEqual([]);
}

test("page tab walk — arrow keys move across the session page's tabs", async ({ page }) => {
  await open(page);
  const stops: Stop[] = [];
  await page.locator("body").focus();
  // D-25: one rail — the rows are the first stops after search and filters.
  await tabTo(page, stops, /^button: .*Work Max/i);
  await page.keyboard.press("Enter");
  // Selecting an Instance focuses its terminal, which keeps Tab for the
  // shell; Ctrl+Shift+Tab leaves it (no keyboard trap, WCAG 2.1.2).
  await expect.poll(async () => (await focused(page)).name).toContain("Terminal input");
  stops.push(await focused(page));
  await expect(page.getByTestId("terminal-leave-hint")).toBeVisible();
  await page.keyboard.press("Control+Shift+Tab");
  stops.push(await focused(page));
  expect(stops.at(-1)!.name).not.toContain("Terminal input");
  // AJ-E1533: a session's page shows only its own tabs, its body first;
  // a terminal's session: its terminal is its Overview (0.1.5).
  await tabTo(page, stops, /^tab: Overview/, { back: true });
  const tablist = page.getByRole("tablist", { name: "Session tabs" });
  const names = await tablist.getByRole("tab").allInnerTexts();
  expect(names).toEqual(["Overview", "Usage", "Processes"]);
  for (const name of [...names.slice(1), names[0]!]) {
    await page.keyboard.press("ArrowRight");
    const stop = await focused(page);
    stops.push(stop);
    await expect(tablist.getByRole("tab", { name, exact: true })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("tabpanel", { name, exact: true })).toBeVisible();
    expect(stop.name).toContain(name);
  }
  record("inspector-tab-walk", stops, [
    "Enter on a row selects it and focuses its terminal; the terminal shows \"⌃Tab leaves the terminal\" and Ctrl+Shift+Tab leaves it.",
    "ArrowRight moves focus and selection across the page's tabs (Terminal, Usage, Processes) and wraps to the first; each panel shows.",
  ]);
});

test("rail — Skip to page first; the list is one stop, ↓ moves in it; a page opened puts the keyboard on it", async ({ page }) => {
  await open(page);
  const stops: Stop[] = [];
  await page.locator("body").focus();
  // deep/11 §9: the skip link is the first stop and goes to the page.
  await page.keyboard.press("Tab");
  stops.push(await focused(page));
  expect(stops[0]!.name).toBe("button: Skip to page");
  await page.keyboard.press("Enter");
  await expect(page.locator("main")).toBeFocused();
  stops.push(await focused(page));
  // From the top again: into the list, then past it — a handful of
  // Tabs, not one per row and control (the 0.2 walk, F11: 84).
  await page.locator("body").focus();
  await tabToEl(page, stops, "[data-testid='rail-list'] [data-rail-item], [data-testid='rail-list'] [data-rail-nav]");
  const inList = () => page.evaluate(() => !!document.querySelector("[data-testid='rail-list']")?.contains(document.activeElement));
  const rows = await page.locator("[data-testid='rail-list'] [data-rail-item]").count();
  expect(rows).toBeGreaterThan(5);
  let tabs = 0;
  while (await inList()) {
    await page.keyboard.press("Tab");
    stops.push(await focused(page));
    tabs += 1;
    expect(tabs, "Tabs to leave the list").toBeLessThanOrEqual(4);
  }
  // Back into the list (Shift+Tab lands on the line it left), then ↓.
  await tabToEl(page, stops, "[data-testid='rail-list'] [data-rail-item], [data-testid='rail-list'] [data-rail-nav]", { back: true, max: 6 });
  const before = (await focused(page)).name;
  await page.keyboard.press("ArrowDown");
  const after = await focused(page);
  stops.push(after);
  expect(after.name).not.toBe(before);
  expect(await inList()).toBe(true);
  // Home's J then Enter opens a page; the row that had focus goes with
  // Home, and the keyboard lands on the page, never the window's body.
  await page.locator("main").focus();
  await page.keyboard.press("Tab");
  stops.push(await focused(page));
  await page.keyboard.press("j");
  stops.push(await focused(page));
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("home")).toHaveCount(0);
  await expect.poll(async () => (await focused(page)).name).not.toBe("(body)");
  stops.push(await focused(page));
  record("rail", stops, [
    "Tab 1 is Skip to page; Enter puts the keyboard on the page (`<main>`).",
    `The list (${rows} rows) is one stop: at most 4 Tabs (its line's row and controls) leave it.`,
    "↓ moves to the next line inside it; Shift+Tab comes back to the line last used.",
    "Home's J then Enter opens the page and the keyboard is on it (not the window's body).",
  ]);
});

test("launch — the sheet opens, traps focus, and Esc returns to New", async ({ page }) => {
  await open(page);
  const stops: Stop[] = [];
  await page.locator("body").focus();
  await tabTo(page, stops, /^button: .*\bNew session$/);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("new-session-sheet")).toBeVisible();
  // Walk the whole sheet: every stop stays inside and shows focus.
  for (let i = 0; i < 15; i++) {
    await page.keyboard.press("Tab");
    const stop = await focused(page);
    stops.push(stop);
    const inside = await page.evaluate(() => !!document.querySelector("[data-testid='new-session-sheet']")?.contains(document.activeElement));
    expect(inside, `focus left the sheet at ${stop.name}`).toBe(true);
  }
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("new-session-sheet")).toHaveCount(0);
  stops.push(await focused(page));
  expect(stops.at(-1)!.name).toMatch(/\bNew session$/);
  record("launch", stops, ["15 Tabs stay inside the sheet (focus trap); Esc closes it and focus returns to New session."]);
});

// D-50 (the maintainer, 2026-10-28): ⌘N opens New session, ⌘⇧N New
// project, ⌥⌘N the notifications; each opens only its own sheet.
test("launch keys — ⌘N New session, ⌘⇧N New project, ⌥⌘N notifications (D-50)", async ({ page }) => {
  await open(page);
  // The keys' handler mounts with the shell (AJ-E806: a test race otherwise).
  await expect(page.getByRole("banner").getByRole("button", { name: "New session", exact: true })).toBeVisible();
  await page.locator("body").focus();
  await page.keyboard.press("ControlOrMeta+n");
  await expect(page.getByTestId("new-session-sheet")).toBeVisible();
  await expect(page.getByRole("heading", { name: "New project" })).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("new-session-sheet")).toHaveCount(0);
  await page.keyboard.press("ControlOrMeta+Alt+KeyN");
  await expect(page.getByTestId("notification-centre")).toBeVisible();
  await expect(page.getByRole("heading", { name: "New project" })).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.keyboard.press("ControlOrMeta+Shift+N");
  await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();
  await expect(page.getByTestId("new-session-sheet")).toHaveCount(0);
});

/** The Work Max session in the rail → its first ranked switch row. The
 * rail's list is one stop on the Tab path; ↓ goes through its rows. */
/** Opens Continue on another account by keyboard; returns what had the
 * keyboard when it opened (where Esc from the switch comes back to). */
async function toSwitchRow(page: Page, stops: Stop[]): Promise<Stop> {
  await page.locator("body").focus();
  await tabToEl(page, stops, "[data-testid='rail-list'] [data-rail-item], [data-testid='rail-list'] [data-rail-nav]");
  await arrowTo(page, stops, /\bon Work Max\b/);
  await page.keyboard.press("Enter");
  await leaveTerminalIfFocused(page, stops);
  const opener = await focused(page);
  // D-25, 0.3.2: ⌘⇧H opens the ranked list in a dialog, the Suggested
  // (else the first) account holding the keyboard once the rows load.
  await page.keyboard.press("ControlOrMeta+Shift+H");
  await expect(page.getByTestId("continue-dialog")).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => Boolean(document.activeElement?.closest("[data-testid='continue-dialog']") && document.activeElement?.hasAttribute("data-row"))))
    .toBe(true);
  stops.push(await focused(page));
  return opener;
}

test("switch, 1-click — popover, confirm, undo window", async ({ page }) => {
  await open(page, allStatuses, { __AIJ_SWITCH_PREFLIGHT__: DOOR_ONECLICK });
  const stops: Stop[] = [];
  await toSwitchRow(page, stops);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("switch-oneclick")).toBeVisible();
  stops.push(await focused(page));
  const inPopover = () => page.evaluate(() => !!document.querySelector("[data-testid='switch-flow']")?.contains(document.activeElement));
  expect(await inPopover(), "opening the switch moves focus into it").toBe(true);
  await tabToEl(page, stops, "[data-testid='switch-confirm']", { max: 20 });
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("switch-undo-toast")).toBeVisible();
  // The dialog is gone; focus lands on Undo instead of the page body.
  await expect(page.getByTestId("switch-undo")).toBeFocused();
  stops.push(await focused(page));
  record("switch-one-click", stops, [
    "Enter on the ranked switch row opens the 1-click popover with focus inside it.",
    "Tab reaches Switch; Enter starts it; focus lands on the toast's Undo (not the page body).",
  ]);
});

test("switch, wizard — three steps by keyboard; Esc returns to where it was opened", async ({ page }) => {
  await open(page, allStatuses, { __AIJ_SWITCH_PREFLIGHT__: DOOR_WIZARD });
  const stops: Stop[] = [];
  // 0.3.2: the row is in Continue on another account's dialog, gone once
  // a row is picked; Esc from the switch comes back to what opened it.
  const invoker = await toSwitchRow(page, stops);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("switch-flow")).toBeVisible();
  stops.push(await focused(page));
  for (const step of ["Step 2 of 3", "Step 3 of 3"]) {
    await tabToEl(page, stops, "[data-testid='switch-continue']", { max: 30 });
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("switch-steps")).toContainText(step);
    stops.push(await focused(page));
  }
  await expect(page.getByTestId("switch-step-confirm")).toBeVisible();
  // The confirm step is reachable; then Esc backs out without switching.
  await tabToEl(page, stops, "[data-testid='switch-confirm']", { max: 30 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("switch-flow")).toHaveCount(0);
  await expect.poll(async () => (await focused(page)).name).toBe(invoker.name);
  const back = await focused(page);
  stops.push(back);
  expect(back.name, "Esc returns focus to what opened the switch's dialog").toBe(invoker.name);
  record("switch-wizard", stops, [
    "Continue moves Step 1 → 2 → 3 (announced politely); the confirm button is reachable.",
    "Esc closes the wizard without switching; focus returns to what had it when ⌘⇧H opened the accounts.",
  ]);
});

test("processes — Stop… opens the confirm sheet; Esc returns to the row", async ({ page }) => {
  await open(page, allStatuses);
  const stops: Stop[] = [];
  await page.locator("body").focus();
  // AJ-E2507: Processes is in your menu (a pill shows while something's
  // left); the menu's lines are reached with the arrows.
  await tabTo(page, stops, /^button: Your menu/);
  await page.keyboard.press("Enter");
  for (let i = 0; i < 12 && !/^menuitem: Processes/.test((await focused(page)).name); i++) {
    await page.keyboard.press("ArrowDown");
  }
  stops.push(await focused(page));
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("processes-page")).toBeVisible();
  // The tree is one Tab stop; arrows move inside it (AJ-E2404).
  await tabToEl(page, stops, "[role='row'][data-kind]");
  for (let i = 0; i < 10; i++) {
    if (await page.evaluate(() => document.activeElement?.getAttribute("data-testid")?.startsWith("process-row-p:8123:") ?? false)) break;
    await page.keyboard.press("ArrowDown");
    await page.waitForTimeout(50);
  }
  const row = await focused(page);
  stops.push(row);
  expect(row.name).toContain("node (vite)");
  await page.keyboard.press("Enter");
  const sheet = page.locator("[data-testid^='kill-sheet-']");
  await expect(sheet).toBeVisible();
  await expect.poll(() => page.evaluate(() => !!document.querySelector("[data-testid^='kill-sheet-']")?.contains(document.activeElement))).toBe(true);
  stops.push(await focused(page));
  // Walk the sheet: focus stays inside it.
  for (let i = 0; i < 10; i++) {
    await page.keyboard.press("Tab");
    stops.push(await focused(page));
    const inside = await page.evaluate(() => !!document.querySelector("[data-testid^='kill-sheet-']")?.contains(document.activeElement));
    expect(inside, `focus left the confirm sheet at ${stops.at(-1)!.name}`).toBe(true);
  }
  await page.keyboard.press("Escape");
  await expect(sheet).toHaveCount(0);
  await expect.poll(async () => (await focused(page)).name).toBe(row.name);
  stops.push(await focused(page));
  record("cleanup-confirm", stops, [
    "Processes is reached from the top bar; Enter on a left-behind row (Stop…) opens the confirm sheet with focus inside.",
    "Tab stays inside the sheet; Esc cancels and returns focus to the row.",
  ]);
});

// AJ-E1022: first run is the onboarding flow (the old account wizard is
// gone): what's here → bring in → sign in later → computer access → Home,
// then Add account's fresh account by keyboard alone.
const FIRST_RUN = {
  __AIJ_PROVIDER_INSTALLS__: [
    { provider: "claude-code", display_name: "Claude Code", install_url: "https://claude.com/download", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [] },
  ],
  __AIJ_SCAN_REPORT__: {
    installs: [
      {
        harness: "claude-code", path: "/Users/you/.claude-work", origin: "home-pattern", login: "logged_out", bytes: 9_000_000,
        last_used_secs: Math.floor(T0 / 1000) - 3600, managed: false, notes: [], signals: ["home-pattern"],
        inference: { purpose: "Work", purpose_confidence: "found", login: "signed-out", login_confidence: "found", activity: "active", activity_confidence: "found", activity_words: "Active this week", keychain_bound: false },
      },
    ],
    probes: [], seeds: [], desktop_apps: [], cancelled: false,
  },
};

test("onboarding — first run end to end by keyboard", async ({ page }) => {
  await open(page, honestEmpty, FIRST_RUN, false);
  await expect(page.getByTestId("first-run-found")).toBeVisible();
  const stops: Stop[] = [];
  const press = async (selector: string, then: string) => {
    await tabToEl(page, stops, selector, { max: 40 });
    await page.keyboard.press("Enter");
    await expect(page.getByTestId(then)).toBeVisible();
  };
  // The found setup is preselected; Tab reaches its checkbox and the button.
  await tabToEl(page, stops, "[data-testid='first-run-row'] input[type='checkbox']", { max: 40 });
  // ADR-0018: used where it is, a signed-in setup needs no sign-in step.
  await press("[data-testid='first-run-bring-in']", "first-run-access");
  await press("[data-testid='first-run-start']", "first-run-figurehead");
  await press("[data-testid='first-run-figurehead-skip']", "home-empty-accounts");
  record("onboarding", stops, [
    "First run: the found setup's checkbox → the placement choice → Bring in → computer access → Continue → the optional Figurehead account → Continue without an account → Home; every control reached by Tab.",
  ]);
});

test("onboarding — Add account's fresh account by keyboard", async ({ page }) => {
  await open(page, honestEmpty, { ...FIRST_RUN, __AIJ_SCAN_REPORT__: { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false } });
  const stops: Stop[] = [];
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Accounts", exact: true }).click();
  await tabToEl(page, stops, "[data-testid='add-account']", { max: 60 });
  await page.keyboard.press("Enter");
  // AJ-E1601: with a service the tool runs through, the chooser comes
  // first; Tab reaches Claude Code's Choose.
  await expect(page.getByRole("dialog", { name: "Add an account: which tool?" })).toBeVisible();
  await tabToEl(page, stops, "[data-testid='provider-card'][data-provider='claude-code'] [data-testid='provider-choose']", { max: 40 });
  await page.keyboard.press("Enter");
  // Nothing new on this Mac: straight to the name, focused and filled in.
  await expect(page.getByTestId("first-run-fresh-name")).toBeFocused();
  await expect(page.getByTestId("first-run-fresh-name")).toHaveValue("Claude Code");
  await page.keyboard.press("Enter"); // submits: Create and sign in
  await expect(page.getByTestId("first-run-signin-row")).toContainText("Claude Code");
  await tabToEl(page, stops, "[data-testid='first-run-signin-row'] button", { max: 40 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("first-run")).toHaveCount(0);
  record("add-account", stops, [
    "Accounts (clicked) → Add account → Claude Code's Choose in the chooser → the name (focused, filled in) → Enter creates it → its Sign in reachable by Tab → Escape closes.",
  ]);
});
