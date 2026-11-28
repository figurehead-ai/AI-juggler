/**
 * AJ-E1545 / AJ-E1546 evidence: the Welcome, each setup-wizard step
 * (Meet Bot in its three standings) and the tour's stops, in both themes
 * at 1280×800, from generic fixtures only. Run by hand with the clips'
 * config (`AIJ_EVIDENCE=1 scripts/record_clips.sh setup-evidence`);
 * writes `docs/qa/evidence/AJ-E1545/` and `AJ-E1546/`. Not a gate.
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses } from "../visual/fixtures";
import { STATES as MODELS } from "../../src/preview/ai-models";

const ROOT = resolve(process.cwd(), "../docs/qa/evidence");
const NOW = 1_728_800_000_000;
const EMPTY = { accounts: [], profiles: [], sessions: [], projects: [] };
const SCAN = {
  installs: ["/Users/you/.claude", "/Users/you/Library/Application Support/ClaudeCodeProfiles/work"].map((path, n) => ({
    harness: "claude-code",
    path,
    origin: n === 0 ? "vendor-default" : "manager-tool",
    login: "logged_in",
    bytes: 30_000_000,
    last_used_secs: Math.floor(NOW / 1000) - 86_400,
    managed: false,
    notes: [],
    signals: [],
  })),
  probes: [],
  seeds: [],
  desktop_apps: ["Claude"],
  cancelled: false,
};
const prompt = (id: string, title: string, why: string) => ({
  id,
  title,
  why,
  system_says: "“AI Juggler” would like to control this computer using accessibility features.",
  click: "Open System Settings",
  if_declined: "Opening a session still brings its app forward; you pick the window yourself.",
  offered_again: "You can turn this on later in Settings › Desktop apps.",
  needed: true,
  status: "not_asked",
  primed: false,
  change_later: "System Settings › Privacy & Security",
  background: "waits",
  never_shown: false,
  dialog_follows: true,
  dialog: { style: "alert", title: "“AI Juggler” would like to control this computer using accessibility features.", body: "Grant access to this application in Privacy & Security settings, located in System Settings.", buttons: ["Deny", "Open System Settings"], click: "Open System Settings" },
});
const PROMPTS = [
  prompt("accessibility", "Jump to a session's own window", "Brings the right terminal window to the front when you open a session."),
  prompt("screen-recording", "See a live preview of each session", "Shows a session's window inside AI Juggler, on this computer only; nothing is recorded or sent."),
];

test.skip(!process.env.AIJ_EVIDENCE, "evidence captures run by hand");

async function open(page: Page, theme: "dark" | "light", fixture: unknown, seams: Record<string, unknown>, firstRun: boolean): Promise<void> {
  await page.clock.setFixedTime(NOW);
  await page.addInitScript(
    ({ fixture: f, seams: s, t, first }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = f;
      for (const [key, value] of Object.entries(s)) w[key] = value;
      localStorage.setItem("aij.theme", t);
      if (!first) {
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      }
    },
    { fixture, seams, t: theme, first: firstRun },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  // F20 (0.5.1): a new install opens on the Welcome; Get started opens the wizard.
  if (firstRun) {
    await expect(page.getByTestId("home-empty-accounts")).toBeVisible();
    await page.getByTestId("welcome-get-started").click();
  }
}

const shot = async (page: Page, task: string, name: string) => {
  mkdirSync(resolve(ROOT, task), { recursive: true });
  await page.mouse.move(1279, 799);
  await page.waitForTimeout(300);
  await page.screenshot({ path: resolve(ROOT, task, `${name}.png`) });
};

for (const theme of ["dark", "light"] as const) {
  test(`welcome and the setup wizard — ${theme}`, async ({ page }) => {
    await open(page, theme, EMPTY, { __AIJ_SCAN_REPORT__: SCAN, __AIJ_OS_PROMPTS__: PROMPTS, ...seedsOf("models") }, true);
    await expect(page.getByTestId("setup-wizard")).toBeVisible();
    await expect(page.getByTestId("first-run-found")).toBeVisible();
    await shot(page, "AJ-E1545", `step1-your-apps-${theme}`);
    await page.getByTestId("setup-skip").click();
    await expect(page.getByTestId("setup-permissions")).toBeVisible();
    await shot(page, "AJ-E1545", `step3-mac-access-${theme}`);
    await page.getByTestId("setup-permission-accessibility-allow").click();
    await page.evaluate(() => {
      const w = window as unknown as { __AIJ_OS_PROMPTS__: Array<{ id: string; status: string }> };
      w.__AIJ_OS_PROMPTS__ = w.__AIJ_OS_PROMPTS__.map((one) => (one.id === "accessibility" ? { ...one, status: "allowed" } : one));
      window.dispatchEvent(new Event("focus"));
    });
    await expect(page.getByTestId("setup-permission-accessibility-allowed")).toBeVisible();
    await page.getByTestId("setup-permission-screen-recording-skip").click();
    await shot(page, "AJ-E1545", `step3-mac-access-allowed-and-skipped-${theme}`);
    await page.getByTestId("setup-next").click();
    await expect(page.getByTestId("engine-setup")).toBeVisible();
    await page.waitForTimeout(500);
    await shot(page, "AJ-E1545", `step4-meet-bot-apple-${theme}`);
    await page.getByTestId("setup-next").click();
    await expect(page.getByTestId("setup-keep")).toBeVisible();
    await shot(page, "AJ-E1545", `step5-keep-running-${theme}`);
    await page.getByTestId("setup-next").click();
    await expect(page.getByTestId("setup-tour")).toBeVisible();
    await shot(page, "AJ-E1545", `end-take-the-tour-${theme}`);
    await page.getByTestId("setup-finish").click();
    await expect(page.getByTestId("home-empty-accounts")).toBeVisible();
    await shot(page, "AJ-E1545", `welcome-${theme}`);
  });

  for (const state of ["noapple", "nolist"] as const) {
    test(`Meet Bot, ${state} — ${theme}`, async ({ page }) => {
      await open(page, theme, EMPTY, { __AIJ_SCAN_REPORT__: SCAN, __AIJ_OS_PROMPTS__: [], ...seedsOf(state) }, true);
      await page.getByTestId("setup-skip").click();
      await expect(page.getByTestId("engine-setup")).toBeVisible();
      await page.waitForTimeout(500);
      await shot(page, "AJ-E1545", `step4-meet-bot-${state}-${theme}`);
    });
  }

  test(`the tour — ${theme}`, async ({ page }) => {
    await open(page, theme, allStatuses, seedsOf("nolist"), false);
    await expect(page.getByTestId("user-menu-button")).toBeVisible();
    await page.getByTestId("user-menu-button").click();
    await page.getByTestId("user-menu-tour").click();
    await expect(page.getByTestId("tour")).toBeVisible();
    await page.waitForTimeout(800);
    await shot(page, "AJ-E1546", `tour-1-bot-no-ai-${theme}`);
    await page.getByTestId("tour-next").click();
    await expect(page.getByTestId("tour")).toHaveAttribute("data-step", "open-now");
    await shot(page, "AJ-E1546", `tour-2-open-now-${theme}`);
    for (let n = 0; n < 3; n += 1) await page.getByTestId("tour-next").click();
    await expect(page.getByTestId("tour")).toHaveAttribute("data-step", "usage");
    await shot(page, "AJ-E1546", `tour-5-usage-${theme}`);
    await page.getByTestId("tour-next").click();
    await expect(page.getByTestId("tour")).toHaveAttribute("data-step", "switch");
    await shot(page, "AJ-E1546", `tour-6-switch-clip-${theme}`);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("tour")).toHaveCount(0);
  });
}

function seedsOf(state: keyof typeof MODELS): Record<string, unknown> {
  const s = MODELS[state]!;
  return { __AIJ_AI_CATALOG__: s.catalog, __AIJ_AI_MODELS__: s.models, __AIJ_AI_JOBS__: s.jobs };
}
