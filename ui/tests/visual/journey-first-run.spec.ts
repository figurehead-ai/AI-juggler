/**
 * D-25 journey: a brand-new install on a Mac that already has Claude Code
 * set up — the default `~/.claude` plus five Claude desktop profiles —
 * walked as the user meets it: the Welcome → the setup wizard's Your apps
 * → bring them in (Sign in is done by itself) → Mac access → Meet Bot →
 * Keep running → Finish → Home, a screenshot per step (AJ-E1545).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { resolve } from "node:path";
// The journey's screenshots are evidence: the gallery's 66-first-run.
const OUT = process.env.AIJ_JOURNEY_OUT ?? resolve(process.cwd(), "../docs/qa/evidence/_gallery/66-first-run");
// A pinned clock (AJ-E1522): the fixture's "last used" times and the page
// both read this instant, so "Used 3 days ago" reads the same every day.
const NOW_MS = 1_728_800_000_000;
const now = () => Math.floor(NOW_MS / 1000);
const install = (path: string, origin: string, purpose: string, days: number) => ({
  harness: "claude-code",
  path,
  origin,
  login: "logged_in",
  bytes: 30_000_000,
  last_used_secs: now() - days * 86_400,
  managed: false,
  notes: [],
  inference: {
    purpose,
    purpose_confidence: "likely",
    login: "signed-in",
    login_confidence: "found",
    activity: days < 7 ? "active" : "dormant",
    activity_confidence: "found",
    activity_words: days < 7 ? "Active this week" : `Dormant ${days} d`,
    keychain_bound: false,
  },
  signals: [origin],
});
const PROFILES = "/Users/you/Library/Application Support/ClaudeCodeProfiles";
const SCAN = {
  installs: [
    install("/Users/you/.claude", "vendor-default", "Main", 2),
    install(`${PROFILES}/work-a`, "manager-tool", "Work a", 0),
    install(`${PROFILES}/work-b`, "manager-tool", "Work b", 0),
    install(`${PROFILES}/work-c`, "manager-tool", "Work c", 1),
    install(`${PROFILES}/client`, "manager-tool", "Client", 3),
    install(`${PROFILES}/acme-max`, "manager-tool", "Personal", 0),
  ],
  probes: [
    { label: "Your standard setup (~/.claude)", health: "ok", detail: "1 found", reason: null },
    { label: "Desktop apps", health: "ok", detail: "Claude", reason: null },
    { label: "Other folders named .claude-*", health: "ok", detail: "none", reason: null },
  ],
  seeds: [],
  desktop_apps: ["Claude"],
  cancelled: false,
};

// 0.5.1 F35 (AJ-E1545 R2): two Claude app profiles open: work-a, paired
// by evidence with its Claude Code folder (one row), and design, whose
// folder isn't known (a row of its own). Neither is ticked for the user
// (D-84); both are counted in Bring in N once ticked.
const PAIRINGS = [
  { id: "app:claude-app:work-a", label: "Claude app · work-a", added: false, certain: true, tool: "claude-code", path: `${PROFILES}/work-a`, running: true,
    folder: { tool: "Claude Code", name: "work-a", path: "~/Library/Application Support/ClaudeCodeProfiles/work-a", by: "running", why: null } },
  { id: "app:claude-app:design", label: "Claude app · design", added: false, certain: false, tool: "claude-code", path: null, running: true,
    folder: { tool: "Claude Code", name: null, path: null, by: null, why: "None of its sessions is open." } },
];

// AJ-E1545 (D-127): what the system says of the permissions this setup
// may need: Accessibility not asked yet; Screen Recording not needed while
// no tool can show a preview, so its card never shows.
const prompt = (id: string, title: string, needed: boolean) => ({
  id,
  title,
  why: "Brings the right terminal window to the front when you open a session.",
  system_says: "“AI Juggler” would like to control this computer using accessibility features.",
  click: "Open System Settings",
  if_declined: "Opening a session still brings its app forward; you pick the window yourself.",
  offered_again: "You can turn this on later in Settings › Desktop apps.",
  needed,
  status: "not_asked",
  primed: false,
  change_later: "System Settings › Privacy & Security",
  background: "waits",
  never_shown: false,
  dialog_follows: true,
  dialog: { style: "alert", title: "“AI Juggler” would like to control this computer using accessibility features.", body: "Grant access to this application in Privacy & Security settings, located in System Settings.", buttons: ["Deny", "Open System Settings"], click: "Open System Settings" },
});
const PROMPTS = [prompt("accessibility", "Jump to a session's own window", true), prompt("screen-recording", "See a live preview of each session", false)];

/** 0.5.1: the Welcome after Finish at rest for its capture. Its clips
 * played on, so a fixed 600 ms wait caught a different frame each run
 * (the gate's 0.2 % against its 0.1 %): each clip is its still poster now
 * (as under Reduce Motion, the clip's own still), its found line drawn and
 * the poster loaded, and nothing on the page moves across several checks
 * (AJ-E2316's wait, as the rail's captures). */
async function welcomeStill(page: Page) {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.getByTestId("welcome-found")).toBeVisible();
  await expect(page.locator("[data-testid^='clip-']:not([data-still])")).toHaveCount(0);
  await expect(page.locator("[data-testid^='clip-'][data-still='true']").first()).toBeVisible();
  let last = "";
  await expect
    .poll(
      async () => {
        const now = await page.evaluate(() => {
          const seen = (el: Element) => {
            const box = el.getBoundingClientRect();
            return box.bottom > 0 && box.top < innerHeight && box.width > 0;
          };
          const posters = Array.from(document.images).filter(seen);
          const parts = Array.from(document.querySelectorAll<HTMLElement>("[data-testid]")).filter(seen);
          return `${posters.every((img) => img.complete && img.naturalWidth > 0)}|${parts.map((el) => `${el.dataset.testid}:${Math.round(el.getBoundingClientRect().top)}:${el.textContent?.length ?? 0}`).join(",")}`;
        });
        const same = now === last && now.startsWith("true|");
        last = now;
        return same;
      },
      { intervals: [200, 250, 250, 300, 400, 500], timeout: 10_000 },
    )
    .toBe(true);
}

/** ADR-0018's other choice, under Your apps' More options in the setup
 * wizard (AJ-E1545; 0.5.1 F47: it sat in the page on the old first run). */
async function separateCopies(page: Page) {
  const more = page.getByTestId("first-run-more-options").locator("button[aria-expanded]").first();
  if ((await more.getAttribute("aria-expanded")) === "false") await more.click();
  await page.getByRole("radio", { name: "Make separate copies" }).check();
}

/** axe: no critical or serious violation on this step (AJ-E1013 gate). */
async function axeClean(page: Page, step: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${step}`).toEqual([]);
}

// 0.5.1 F51: every state on the pinned clock, so the fixture's times
// read as a real setup's ("Used today", "3 days ago"); the states after
// the first read the real clock and said "Used 772 days ago".
test.beforeEach(async ({ page }) => {
  await page.clock.setFixedTime(NOW_MS);
});

test("first run on a Mac with Claude Code already set up", gate("66-first-run"), async ({ page }) => {
  await page.addInitScript((scan) => {
    const w = window as unknown as Record<string, unknown>;
    w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
    w.__AIJ_SCAN_REPORT__ = scan.SCAN;
    w.__AIJ_PLACES__ = { places: [], offers: [], pairings: scan.PAIRINGS };
    w.__AIJ_OS_PROMPTS__ = scan.PROMPTS;
    // Anything typed into a field (the budget allows none).
    w.__AIJ_TYPED__ = 0;
    document.addEventListener("input", (event) => {
      const target = event.target as HTMLInputElement | null;
      if (target && (target.type === "text" || target.tagName === "TEXTAREA")) {
        (window as unknown as { __AIJ_TYPED__: number }).__AIJ_TYPED__ += 1;
      }
    }, true);
  }, { SCAN, PAIRINGS, PROMPTS });
  // AJ-E305's first-install budget, kept for the new first run: at most
  // five primary clicks and nothing typed, from opening to Home.
  let primaryClicks = 0;
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  // F20 (0.5.1): the Welcome first; Get started opens the wizard.
  await page.getByTestId("welcome-get-started").click();
  primaryClicks += 1;
  const found = page.getByTestId("first-run-found");
  await expect(found).toBeVisible();
  await page.screenshot({ path: `${OUT}/found-1280x800-dark.png` });
  await axeClean(page, "what's here");
  // AJ-E1545 R2: the wizard's Your apps step, its own words.
  await expect(found.getByRole("heading", { level: 1 })).toHaveText("Which AI apps do you use?");
  await expect(found).toContainText("We found these on your Mac. Tick the ones AI Juggler should show.");
  // F35: each profile a row in Claude Code's group, Desktop chip, no
  // separate block; work-a's folder is its row, so 5 are ticked, not 6.
  for (const name of ["Claude app · work-a", "Claude app · design"]) {
    const row = found.getByTestId("first-run-row").filter({ hasText: name });
    await expect(row.getByTestId("first-run-chip").first()).toHaveText("Desktop");
    await expect(row.getByRole("checkbox")).not.toBeChecked();
  }
  await expect(found.getByTestId("first-run-row")).toHaveCount(7);
  await expect(page.getByTestId("first-run-desktop")).toHaveCount(0);
  await expect(page.getByTestId("first-run-bring-in")).toHaveText("Bring in 5 apps");
  await found.getByTestId("first-run-row").filter({ hasText: "Claude app · design" }).getByRole("checkbox").check();
  await found.getByTestId("first-run-row").filter({ hasText: "Claude app · work-a" }).getByRole("checkbox").check();
  await expect(page.getByTestId("first-run-bring-in")).toHaveText("Bring in 7 apps");
  const looked = page.getByRole("button", { name: "Where we looked" });
  if (await looked.count()) await looked.click();
  // The page scrolls inside the window, so bring the list into view (the
  // footer stays pinned since the 0.4 review; the capture showed the top).
  await page.getByTestId("first-run-looked").scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${OUT}/where-we-looked-1280x800-dark.png`, fullPage: true });
  await page.getByTestId("first-run-bring-in").click();
  primaryClicks += 1;
  // ADR-0018: used where they are, signed-in setups stay signed in, so the
  // wizard's Sign in is done by itself (AJ-E1545) and Bring in lands on
  // the Mac access step: one card per permission this setup needs.
  const access = page.getByTestId("setup-permissions");
  await expect(access).toBeVisible();
  await expect(page.getByTestId("first-run-signin")).toHaveCount(0);
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${OUT}/after-bring-in-1280x800-dark.png` });
  await axeClean(page, "computer access");
  // Screen Recording isn't asked for while no tool can show a preview
  // (the backend says it isn't needed; the step shows only what is).
  await expect(access).not.toContainText("Screen Recording");
  await expect(page.getByTestId("setup-permission-screen-recording")).toHaveCount(0);
  await page.getByTestId("setup-permission-accessibility-allow").click();
  // The system said yes: the card reads Allowed once it is read again.
  await page.evaluate(() => {
    const w = window as unknown as { __AIJ_OS_PROMPTS__: Array<{ id: string; status: string }> };
    w.__AIJ_OS_PROMPTS__ = w.__AIJ_OS_PROMPTS__.map((one) => (one.id === "accessibility" ? { ...one, status: "allowed" } : one));
    window.dispatchEvent(new Event("focus"));
  });
  await expect(page.getByTestId("setup-permission-accessibility-allowed")).toBeVisible();
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${OUT}/access-one-allowed-1280x800-dark.png` });
  // Meet Bot, then Keep running, then the tour's offer: one Next each.
  for (const next of ["engine-setup", "setup-keep", "setup-tour"]) {
    await page.getByTestId("setup-next").click();
    primaryClicks += 1;
    await expect(page.getByTestId(next)).toBeVisible();
  }
  await page.getByTestId("setup-finish").click();
  primaryClicks += 1;
  await expect(page.getByTestId("setup-wizard")).toHaveCount(0);
  // AJ-E1545's wizard: Get started, Bring in, one Next per step after Mac
  // access, and Finish (AJ-E305's budget was five for the old three steps).
  expect(primaryClicks).toBeLessThanOrEqual(6);
  expect(await page.evaluate(() => (window as unknown as { __AIJ_TYPED__: number }).__AIJ_TYPED__)).toBe(0);
  await welcomeStill(page);
  await page.screenshot({ path: `${OUT}/home-after-first-run-1280x800-dark.png` });
  await axeClean(page, "Home");
});

test("first run, choosing separate copies: each copy signs in once, then the wizard goes on", async ({ page }) => {
  await page.addInitScript((scan) => {
    const w = window as unknown as Record<string, unknown>;
    w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
    w.__AIJ_SCAN_REPORT__ = scan.SCAN;
    w.__AIJ_OS_PROMPTS__ = scan.PROMPTS;
  }, { SCAN, PROMPTS });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await page.getByTestId("welcome-get-started").click();
  await expect(page.getByTestId("first-run-found")).toBeVisible();
  await separateCopies(page);
  await expect(page.getByTestId("first-run-found")).toContainText("makes its own copy of each");
  await page.screenshot({ path: `${OUT}/found-copies-1280x800-dark.png` });
  await axeClean(page, "copies chosen");
  await page.getByTestId("first-run-bring-in").click();
  const signin = page.getByTestId("first-run-signin");
  await expect(signin).toBeVisible();
  // 0.5.1 F48: it stays on Sign in (it fell back to Your apps a moment later).
  await expect(page.getByTestId("setup-wizard")).toHaveAttribute("data-step", "signin");
  await page.waitForTimeout(600);
  await expect(signin).toBeVisible();
  await page.screenshot({ path: `${OUT}/after-bring-in-copies-1280x800-dark.png` });
  await axeClean(page, "sign-in");
  // 0.5.1 F50: the step's one primary last in the wizard's foot, after
  // Skip this step; the page has no skip of its own.
  await expect(page.getByTestId("setup-foot-end")).toContainText("Sign in to all 6");
  await expect(page.getByTestId("first-run-signin-next")).toHaveCount(0);
  // AJ-E1545: Sign in skipped, the setup wizard goes on: Mac access, Meet
  // Bot, Keep running, then Finish (0.5.1 F47: the old first run's own
  // access step is gone).
  await page.getByTestId("setup-skip").click();
  await expect(page.getByTestId("setup-wizard")).toHaveAttribute("data-step", "access");
  await expect(page.getByTestId("setup-permissions")).toBeVisible();
  for (const [next, step] of [["engine-setup", "bot"], ["setup-keep", "keep"], ["setup-tour", "tour"]] as const) {
    await page.getByTestId("setup-next").click();
    await expect(page.getByTestId("setup-wizard")).toHaveAttribute("data-step", step);
    await expect(page.getByTestId(next)).toBeVisible();
  }
  await page.getByTestId("setup-finish").click();
  await expect(page.getByTestId("setup-wizard")).toHaveCount(0);
});

// ADR-0018 §5: one setup that can't come in never blocks the rest — the
// user decides (Continue, Try again, or use it where it is instead).
for (const theme of ["dark", "light"] as const) {
  test(`first run — one copy fails, the user decides — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ scan, theme }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
        w.__AIJ_SCAN_REPORT__ = scan;
        w.__AIJ_IMPORT_FAIL_PATHS__ = [scan.installs[0].path];
        localStorage.setItem("aij.theme", theme);
      },
      { scan: SCAN, theme },
    );
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByTestId("welcome-get-started").click();
    await separateCopies(page);
    await page.getByTestId("first-run-bring-in").click();
    const outcome = page.getByTestId("first-run-outcome");
    await expect(outcome).toContainText("5 brought in. 1 couldn't come in.");
    await expect(outcome.getByRole("button", { name: "Use it where it is instead" })).toBeVisible();
    await expect(outcome.getByRole("button", { name: "Continue with 5 apps" })).toBeVisible();
    await page.screenshot({ path: `${OUT}/one-failed-1280x800-${theme}.png`, fullPage: true });
    await axeClean(page, `one failed (${theme})`);
  });
}


// The 0.0.12 report: Screen Recording allowed in System Settings, the
// card never turned to Allowed (macOS answers once per process). After
// Reopen AI Juggler, the resume marker opens the setup wizard back at Mac
// access (AJ-E1545; 0.5.1 F47: the old first run's own access step and
// its Reopen card are gone): what was allowed says Allowed; one the system
// was asked about (macOS won't say the answer) offers Allow again; then
// the wizard goes on to Meet Bot.
const resumedPrompts = [
  { ...prompt("accessibility", "Jump to a session's own window", true), status: "allowed" },
  {
    ...prompt("screen-recording", "See a live preview of each session", true),
    status: "asked",
    why: "Shows a session's window inside AI Juggler, on this Mac only; nothing is recorded or sent.",
    system_says: "“AI Juggler” would like to record this computer's screen and audio.",
    if_declined: "Sessions show without a live preview of their window.",
    dialog: { style: "alert", title: "“AI Juggler” would like to record this computer's screen and audio.", body: "Grant access to this application in Privacy & Security settings, located in System Settings.", buttons: ["Deny", "Open System Settings"], click: "Open System Settings" },
  },
];
for (const theme of ["dark", "light"] as const) {
  test(`first run — back at Mac access after a reopen — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ scan, prompts, theme }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
        w.__AIJ_SCAN_REPORT__ = scan;
        w.__AIJ_OS_PROMPTS__ = prompts;
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.firstRun.resumeAccess", "1");
      },
      { scan: SCAN, prompts: resumedPrompts, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    const wizard = page.getByTestId("setup-wizard");
    await expect(wizard).toHaveAttribute("data-step", "access");
    await expect(page.getByTestId("setup-permission-accessibility-allowed")).toContainText("Allowed");
    const asked = page.getByTestId("setup-permission-screen-recording");
    await expect(asked).toHaveAttribute("data-state", "asked");
    await expect(asked.getByTestId("setup-permission-screen-recording-allow")).toBeVisible();
    await page.screenshot({ path: `${OUT}/access-resumed-1280x800-${theme}.png`, fullPage: true });
    await axeClean(page, `Mac access, resumed (${theme})`);
    await page.getByTestId("setup-next").click();
    await expect(wizard).toHaveAttribute("data-step", "bot");
  });
}
