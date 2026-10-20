/**
 * 0.0.7 dogfood: "any actions that are clickable are not showing hand
 * mouse icon". The shared Button and a handful of rows set
 * `cursor-default`, so a mouse user never saw what was clickable. Every
 * enabled control on every main screen — anything with a click handler,
 * and every button, link, summary, select and checkbox row — must show
 * the pointing hand.
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses } from "./fixtures";

const T0 = 1_728_800_000_000;

async function seed(page: Page, fixture: unknown = allStatuses, seams: Record<string, unknown> = {}) {
  await page.addInitScript(
    ({ fixture, t0, seams }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [key, value] of Object.entries(seams)) w[key] = value;
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, t0: T0, seams },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

/** Enabled, visible clickable things whose cursor isn't the hand. */
async function notPointing(page: Page): Promise<{ count: number; wrong: string[]; disabledHands: string[] }> {
  return page.evaluate(() => {
    const disabledHands = [...document.querySelectorAll<HTMLElement>("button:disabled, [aria-disabled='true'], select:disabled")]
      .filter((el) => el.getBoundingClientRect().width > 0 && getComputedStyle(el).cursor === "pointer")
      .map((el) => `${el.tagName.toLowerCase()} "${(el.getAttribute("aria-label") ?? el.innerText ?? "").trim().slice(0, 40)}"`);
    const native =
      'button, [role="button"], a[href], summary, select, label:has(> input[type="checkbox"]), label:has(> input[type="radio"])';
    const wrong: string[] = [];
    let count = 0;
    for (const el of document.querySelectorAll<HTMLElement>("body *")) {
      const props = Object.keys(el).find((key) => key.startsWith("__reactProps"));
      const handler = props ? (el as unknown as Record<string, { onClick?: unknown }>)[props]?.onClick : undefined;
      if (!handler && !el.matches(native)) continue;
      if ((el as HTMLButtonElement).disabled || el.getAttribute("aria-disabled") === "true") continue;
      if (el.closest("[inert], [aria-hidden='true']")) continue;
      const box = el.getBoundingClientRect();
      if (box.width === 0 || box.height === 0) continue;
      // A click handler that only stops propagation on a container (a
      // dialog frame, a menu panel) isn't a control.
      if (!el.matches(native) && el.querySelector(native) && !el.getAttribute("tabindex")) continue;
      count += 1;
      if (getComputedStyle(el).cursor !== "pointer") {
        wrong.push(`${el.tagName.toLowerCase()} "${(el.getAttribute("aria-label") ?? el.innerText ?? "").trim().slice(0, 40)}"`);
      }
    }
    return { count, wrong, disabledHands };
  });
}

async function expectHands(page: Page, where: string) {
  const { count, wrong, disabledHands } = await notPointing(page);
  expect(count, `${where}: some controls were checked`).toBeGreaterThan(3);
  expect(wrong, `${where}: clickable without the hand cursor`).toEqual([]);
  expect(disabledHands, `${where}: a disabled control shows the hand`).toEqual([]);
}

test("every clickable control shows the hand — the main screens", async ({ page }) => {
  await seed(page);
  const nav = page.getByRole("navigation", { name: "Main" });
  await expect(nav).toBeVisible();
  await expectHands(page, "Home");
  for (const name of ["Accounts", "History", "Settings"]) {
    await nav.getByRole("button", { name, exact: true }).click();
    await page.waitForTimeout(150);
    await expectHands(page, name);
  }
  for (const name of ["Usage", "Fleet", "Cleanup"]) {
    await nav.getByRole("button", { name: "More", exact: true }).click();
    await page.locator("#nav-more").getByRole("button", { name: new RegExp(`^${name}\\b`) }).click();
    await page.waitForTimeout(150);
    await expectHands(page, name);
  }
});

test("every clickable control shows the hand — menus and the legend", async ({ page }) => {
  await seed(page);
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "More", exact: true }).click();
  await expectHands(page, "More menu");
  await page.keyboard.press("Escape");
  await page.getByTestId("rail").getByRole("button", { name: "Indicator legend" }).click();
  await expectHands(page, "legend open");
});

// AJ-E1021 / AJ-E1023: Home's triage controls (Resume, Mark reviewed,
// Show all) and Add account's tool choice show the hand too.
test("every clickable control shows the hand — Home's triage and the tool choice", async ({ page }) => {
  const parked = {
    ...allStatuses,
    sessions: allStatuses.sessions.map((session) =>
      session.status === "quota_blocked" ? { ...session, parked: "measured", parked_resets_at_ms: T0 - 60_000 } : session,
    ),
  };
  await seed(page, parked, {
    __AIJ_PROVIDER_INSTALLS__: [
      { provider: "claude-code", display_name: "Claude Code", install_url: "https://claude.com/download", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [], verified_against: ["2.1.x"], kind: "terminal", single_instance_note: null },
      { provider: "codex", display_name: "Codex", install_url: "https://developers.openai.com/codex", status: "installed", version: "0.9.4", binary: "/opt/homebrew/bin/codex", notes: [], verified_against: [], kind: "terminal", single_instance_note: null },
    ],
  });
  await expect(page.getByTestId("home-parked").getByRole("button", { name: /^Resume/ }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: /^Mark reviewed/ }).first()).toBeVisible();
  await expectHands(page, "Home with parked and ended sessions");
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Accounts", exact: true }).click();
  await page.getByTestId("add-account").click();
  await expect(page.getByTestId("provider-cards")).toBeVisible();
  await expectHands(page, "Add account's tool choice");
});

// AJ-E1010: every control of the update UI — About & updates in each
// state with buttons (available, failed with Details, a failed check,
// ready to restart) and both prompts on Home — shows the hand.
test("every clickable control shows the hand — the update UI", async ({ page }) => {
  const base = {
    current_version: "0.1.0", channel: "beta", edition: "main", app_name: "AI Juggler", channels: ["stable", "beta"],
    configured: true, check_on_launch: true, state: "idle", available: null, progress: null, error: null,
    error_kind: null, error_detail: null, restart_wait: null, backup_path: null, checked_at_ms: null,
  };
  const available = { version: "0.2.0", date_ms: null, notes: "## New\n- **History**: find any past session." };
  const states: Record<string, Record<string, unknown>> = {
    available: { ...base, state: "available", available },
    failed: {
      ...base, state: "failed", available, error: "Not installed. AI Juggler 0.1.0 is untouched.",
      error_kind: "signature", error_detail: "Invalid signature",
    },
    "check-failed": { ...base, state: "failed", error: "Couldn't reach the update server.", error_kind: "network", error_detail: "x" },
    "restart-ready": { ...base, state: "installed", available, progress: 100 },
  };
  for (const [name, status] of Object.entries(states)) {
    await seed(page, allStatuses, { __AIJ_UPDATE_STATUS__: status });
    const prompt = name === "available" ? "update-notice" : name === "restart-ready" ? "update-ready" : null;
    if (prompt) {
      await expect(page.getByTestId(prompt)).toBeVisible();
      await expectHands(page, `Home with the update ${prompt}`);
    }
    await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByTestId("settings-updates").evaluate((card) => card.scrollIntoView({ block: "start" }));
    await expect(page.getByTestId("beta-label")).toBeVisible();
    await expectHands(page, `About & updates — ${name}`);
    const pointer = await page.evaluate(() => {
      const card = document.querySelector("[data-testid='settings-updates']")!;
      return [...card.querySelectorAll<HTMLElement>("button:not(:disabled), summary, label:has(> input)")].map(
        (el) => `${el.textContent?.trim().slice(0, 30)}: ${getComputedStyle(el).cursor}`,
      );
    });
    expect(pointer.length, `${name}: the card has controls`).toBeGreaterThan(2);
    expect(pointer.filter((line) => !line.endsWith("pointer")), name).toEqual([]);
  }
});
