import { expect, test } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, honestEmpty, goTo, expectRail } from "./fixtures";

/**
 * AJ-E306 quality gates — the three daily flows as click-count
 * contracts: (c) add an account for another tool ≤ 4 PRIMARY clicks,
 * (d) add Account ≤ 3, (e) launch ≤ 2. Since AJ-E1022/AJ-E1023 both (c)
 * and (d) start at Add account (the tool choice appears only when more
 * than one tool is installed) and count to the account's sign-in. Primary activations are counted by the
 * spec's own clicks; the funnel seam records each flow's steps.
 * Also: the launch sheet never uses "Profile" as a noun.
 */

const GALLERY = resolve(process.cwd(), "../docs/qa/evidence/_gallery");

test.beforeAll(() => {
  mkdirSync(resolve(GALLERY, "38-flows-cde"), { recursive: true });
});

test("flow (c): an account for another tool ≤ 4 primary clicks", async ({ page }) => {
  await page.addInitScript(
    ({ fixture }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "installed",
          version: "2.1.296",
          binary: "/usr/local/bin/claude",
          notes: [],
          verified_against: ["2.0.x", "2.1.x"],
          kind: "terminal",
          single_instance_note: null,
        },
        {
          provider: "codex",
          display_name: "Codex",
          install_url: "https://developers.openai.com/codex",
          status: "installed",
          version: "0.9.4",
          binary: "/opt/homebrew/bin/codex",
          notes: [],
          verified_against: [],
          kind: "terminal",
          single_instance_note: null,
        },
      ];
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      const log: string[] = [];
      w.__funnelLog = log;
      window.addEventListener("aij:funnel", (event) => {
        const detail = (event as CustomEvent<{ flow: string; step: string }>).detail;
        log.push(`${detail.flow}:${detail.step}`);
      });
    },
    { fixture: honestEmpty },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await goTo(page, "Accounts");

  let primaryClicks = 0;
  // Click 1: Add account — two tools are installed, so it asks which.
  await page.getByTestId("add-account").click();
  primaryClicks += 1;
  await expect(page.getByTestId("provider-cards")).toBeVisible();
  // Click 2: the installed Codex card.
  await page
    .locator("[data-testid='provider-card'][data-status='installed'] button[data-testid='provider-choose']")
    .nth(1)
    .click();
  primaryClicks += 1;
  // Add account for Codex: nothing of Codex's on this Mac yet, so it
  // opens at a fresh account, its name filled in (0 typed).
  await expect(page.getByTestId("first-run")).toHaveAttribute("data-mode", "add");
  await expect(page.getByTestId("first-run-fresh-name")).toHaveValue("Codex");
  // Click 3: Create and sign in.
  await page.getByTestId("first-run-fresh-create").click();
  primaryClicks += 1;
  // Click 4: Codex's own sign-in.
  const row = page.getByTestId("first-run-signin-row");
  await expect(row).toContainText("Codex");
  await row.getByRole("button", { name: /^Sign in/ }).click();
  primaryClicks += 1;
  expect(primaryClicks).toBeLessThanOrEqual(4);
  const funnel = (await page.evaluate(() =>
    (window as unknown as { __funnelLog: string[] }).__funnelLog,
  )) as string[];
  expect(funnel).toContain("c-add-provider:open");
  expect(funnel).toContain("c-add-provider:choose");
});

test("flow (d): add an Account ≤ 3 primary clicks (one tool: no choice, no gate)", async ({
  page,
}) => {
  await page.addInitScript(
    ({ fixture }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "installed",
          version: "2.1.296",
          binary: "/usr/local/bin/claude",
          notes: [],
        },
      ];
      w.__AIJ_SCAN_REPORT__ = {
        installs: [],
        probes: [],
        seeds: [],
        desktop_apps: [],
        cancelled: false,
      };
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await goTo(page, "Accounts");

  let primaryClicks = 0;
  // Click 1: Add account — one tool installed: no choice, no gate.
  await page.getByTestId("add-account").click();
  primaryClicks += 1;
  await expect(page.getByTestId("provider-cards")).toHaveCount(0);
  // Nothing new on this Mac: straight to a fresh account, the name
  // PRE-FILLED (0 typed).
  await expect(page.getByTestId("first-run-fresh")).toBeVisible();
  await expect(page.getByTestId("first-run-fresh-name")).toHaveValue("Claude Code");
  // Click 2: Create and sign in.
  await page.getByTestId("first-run-fresh-create").click();
  primaryClicks += 1;
  // Click 3: the account's own sign-in.
  const row = page.getByTestId("first-run-signin-row");
  await expect(row).toContainText("Claude Code");
  await row.getByRole("button", { name: /^Sign in/ }).click();
  primaryClicks += 1;
  expect(primaryClicks).toBeLessThanOrEqual(3);
});

test("flow (e): launch ≤ 2 clicks; Profile never a sheet noun; verdict words not %", async ({
  page,
}) => {
  await page.addInitScript(
    ({ fixture }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_RANKING__ = [
        {
          account: "claude-work",
          score: 0.9,
          top_factors: [],
          factors: [],
          headroom_unknown: false,
          confidence: "Measured",
          suggested: true,
          admission: "tight",
        },
      ];
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: allStatuses },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expectRail(page); // D-25: the list, or its icon column when narrow
  // The launch sheet opens (⌘N opens New project; the header New
  // button opens the launch sheet — flow (e)'s surface).
  await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
  await expect(page.getByRole("heading", { name: "New session" })).toBeVisible();

  let clicks = 0;
  // Click 1: the recent dir tile (New session tiles read
  // "name · N prior sessions · accounts").
  await page
    .getByRole("button", { name: /ai-juggler.*prior sessions/s })
    .click();
  clicks += 1;
  const preselected = page.getByTestId("launch-preselected");
  await expect(preselected).toBeVisible();
  await expect(preselected).toContainText("Tight — this session may use most of the room left");
  // Words, never a fabricated percentage (Req.8 regression).
  await expect(page.getByTestId("launch-sheet-accounts")).not.toContainText("%");
  // Click 2: Launch →.
  await page.getByTestId("launch-now").click();
  clicks += 1;
  await expect(page.getByText("Preview mode — no backend connected.")).toBeVisible();
  expect(clicks).toBeLessThanOrEqual(2);
  // The D-02/D-10 noun law: "Profile" never names a sheet thing.
  const sheetText = await page
    .getByRole("heading", { name: "New session" })
    .locator("xpath=ancestor::div[3]")
    .innerText();
  expect(sheetText).not.toMatch(/\bProfile\b/);
  expect(sheetText).toContain("Launch settings");
});
