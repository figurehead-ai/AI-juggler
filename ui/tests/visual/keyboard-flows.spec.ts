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
import { allStatuses, cleanupFixture, DOOR_ONECLICK, DOOR_WIZARD, honestEmpty, type FixtureSet } from "./fixtures";

// D-25: a fresh install opens first run; these captures document the
// Add-account wizard, so they open it the way Add account does.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    (window as unknown as { __AIJ_FIRST_RUN__?: string }).__AIJ_FIRST_RUN__ = "wizard";
  });
});

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

test("inspector tab walk — arrow keys move across all six tabs", async ({ page }) => {
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
  await tabTo(page, stops, /^tab: Overview/, { back: true });
  const names = ["Usage", "History", "Quality / Performance", "Processes", "Actions"];
  for (const name of names) {
    await page.keyboard.press("ArrowRight");
    const stop = await focused(page);
    stops.push(stop);
    await expect(page.getByRole("tab", { name, exact: true })).toHaveAttribute("aria-selected", "true");
    expect(stop.name).toContain(name);
  }
  record("inspector-tab-walk", stops, [
    "Enter on a row selects it and focuses its terminal; the terminal shows \"⌃Tab leaves the terminal\" and Ctrl+Shift+Tab leaves it.",
    "ArrowRight moves focus and selection across the six tabs; each panel shows.",
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

/** The Work Max session in the rail → its first ranked switch row. */
async function toSwitchRow(page: Page, stops: Stop[]) {
  await page.locator("body").focus();
  await tabTo(page, stops, /Work Max/);
  await page.keyboard.press("Enter");
  await leaveTerminalIfFocused(page, stops);
  // D-25: the ranked list is folded; ⌘⇧H opens it and focuses it.
  await page.keyboard.press("ControlOrMeta+Shift+H");
  await expect(page.locator("[data-testid='handover-section'] > summary")).toBeFocused();
  stops.push(await focused(page));
  await tabToEl(page, stops, "[data-testid='handover-section'] button[data-row]:not(:disabled)", { max: 120 });
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

test("switch, wizard — three steps by keyboard; Esc returns to the row", async ({ page }) => {
  await open(page, allStatuses, { __AIJ_SWITCH_PREFLIGHT__: DOOR_WIZARD });
  const stops: Stop[] = [];
  await toSwitchRow(page, stops);
  const invoker = await focused(page);
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
  expect(back.name, "Esc returns focus to the row that opened the wizard").toBe(invoker.name);
  record("switch-wizard", stops, [
    "Continue moves Step 1 → 2 → 3 (announced politely); the confirm button is reachable.",
    "Esc closes the wizard without switching; focus returns to the switch row that opened it.",
  ]);
});

test("cleanup — Terminate opens the confirm sheet; Esc returns to the row", async ({ page }) => {
  await open(page, allStatuses, { __AIJ_CLEANUP__: cleanupFixture("needs-attention") });
  const stops: Stop[] = [];
  await page.locator("body").focus();
  // D-25: Cleanup waits under More (a pill shows while something's left).
  await tabTo(page, stops, /^button: More$/);
  await page.keyboard.press("Enter");
  await tabTo(page, stops, /^button: Cleanup/);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("cleanup-centre")).toBeVisible();
  await tabTo(page, stops, /^button: Terminate…$/);
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
  await expect.poll(async () => (await focused(page)).name).toBe("button: Terminate…");
  stops.push(await focused(page));
  record("cleanup-confirm", stops, [
    "Cleanup is reached from the top bar; the leftover's Terminate… opens the confirm sheet with focus inside.",
    "Tab stays inside the sheet; Esc cancels and returns focus to Terminate….",
  ]);
});

const FIRST_RUN = {
  __AIJ_PROVIDER_INSTALLS__: [
    { provider: "claude-code", display_name: "Claude Code", install_url: "https://claude.com/download", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [] },
  ],
  __AIJ_SCAN_REPORT__: { installs: [], probes: [], seeds: ["/Users/you/dev-api"], desktop_apps: [], cancelled: false },
  __AIJ_RECOMMENDATION__: {
    items: [
      { provider: "claude-code", label: "Work", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-work" },
      { provider: "claude-code", label: "Side", action: "import", rationale: "Active — you'll sign in after import", default_on: true, path: "/Users/you/.claude-side" },
    ],
    summary: "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
    start_here: "/Users/you/dev-api",
  },
};

test("onboarding — the first-run wizard end to end by keyboard", async ({ page }) => {
  await open(page, honestEmpty, FIRST_RUN, false);
  await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
  const stops: Stop[] = [];
  const press = async (testId: string, then: string) => {
    await tabToEl(page, stops, `[data-testid='${testId}']`, { max: 40 });
    await page.keyboard.press("Enter");
    await expect(page.getByTestId(then)).toBeVisible();
  };
  await press("install-continue", "proposal-teaser");
  await press("use-recommended", "proposal-block");
  // The pack-policy question: reach the radio group, choose with Space.
  await tabToEl(page, stops, "[data-testid='policy-panel'] input[type='radio']", { max: 40 });
  await page.keyboard.press("Space");
  await expect(page.getByTestId("accept-setup")).toBeEnabled();
  await press("accept-setup", "signin-queue");
  for (let card = 0; card < 2; card++) {
    await tabToEl(page, stops, "[data-testid='signin-skip']", { max: 40 });
    await page.keyboard.press("Enter");
  }
  await expect(page.getByTestId("summary-exits")).toBeVisible();
  await tabToEl(page, stops, "[data-testid='launch-first-session']", { max: 40 });
  record("onboarding", stops, [
    "Gate → recommended setup → pack policy (radio, Space) → accept → both sign-ins skipped → summary; every control reached by Tab.",
    "The final Launch first session button is reachable by keyboard.",
  ]);
});
