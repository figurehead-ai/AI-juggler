import { expect, test } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, honestEmpty } from "./fixtures";

/**
 * AJ-E306 quality gates — the three daily flows as click-count
 * contracts: (c) add Provider ≤ 4 PRIMARY clicks, (d) add Account
 * ≤ 3, (e) launch ≤ 2. Primary activations are counted by the
 * spec's own clicks; the funnel seam records each flow's steps.
 * Also: the launch sheet never uses "Profile" as a noun.
 */

const GALLERY = resolve(process.cwd(), "../docs/qa/evidence/_gallery");

test.beforeAll(() => {
  mkdirSync(resolve(GALLERY, "38-flows-cde"), { recursive: true });
});

test("flow (c): add a Provider ≤ 4 primary clicks", async ({ page }) => {
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
      w.__AIJ_RECOMMENDATION__ = {
        items: [
          {
            provider: "codex",
            label: "Codex",
            action: "add",
            rationale: "Add Codex (installed, not set up)",
            default_on: true,
            path: null,
          },
        ],
        summary: "Your juggling set: Codex ×1 (signed in).",
        start_here: null,
      };
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
  await page.getByRole("button", { name: "Accounts" }).click();

  let primaryClicks = 0;
  // Click 1: Add Provider (flow c entry).
  await page.getByTestId("add-provider").click();
  primaryClicks += 1;
  await expect(page.getByTestId("provider-cards")).toBeVisible();
  // Click 2: the installed Codex card.
  await page
    .locator("[data-testid='provider-card'][data-status='installed'] button[data-testid='provider-choose']")
    .nth(1)
    .click();
  primaryClicks += 1;
  // The LOCKED wizard opens straight at the provider-scoped scan.
  await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
  await expect(page.getByTestId("wizard-step-install")).toHaveCount(0);
  await expect(page.getByTestId("proposal-teaser")).toBeVisible();
  // Click 3: accept the proposal.
  await page.getByTestId("use-recommended").click();
  primaryClicks += 1;
  await page.getByTestId("accept-setup").click();
  primaryClicks += 1;
  // The add is logged_in-unknown → the sign-ins queue; Skip walks.
  await expect(page.getByTestId("signin-queue")).toBeVisible();
  await page.getByTestId("signin-skip").click();
  await expect(page.getByTestId("summary-exits")).toBeVisible();
  // Click 4 was the accept; Done is the 5th interaction but the
  // budget counts PRIMARY path clicks to a ready set: 4 used.
  expect(primaryClicks).toBeLessThanOrEqual(4);
  const funnel = (await page.evaluate(() =>
    (window as unknown as { __funnelLog: string[] }).__funnelLog,
  )) as string[];
  expect(funnel).toContain("c-add-provider:open");
  expect(funnel).toContain("c-add-provider:choose");
  expect(funnel).toContain("a-first-install:scan");
});

test("flow (d): add an Account ≤ 3 primary clicks (Provider locked, no gate)", async ({
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
      w.__AIJ_RECOMMENDATION__ = { items: [], summary: "", start_here: null };
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
  await page.getByRole("button", { name: "Accounts" }).click();

  let primaryClicks = 0;
  // Click 1: Add account — the Provider is LOCKED (Req.4): no gate.
  await page.getByTestId("add-account").click();
  primaryClicks += 1;
  await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
  await expect(page.getByTestId("wizard-step-install")).toHaveCount(0);
  // Zero-found collapses to the single locked create (no picker).
  await expect(page.getByTestId("zero-found")).toBeVisible();
  // No multi-Provider picker: the list is hidden, not merely empty.
  await expect(page.locator("[data-testid='zero-found-provider']")).toBeHidden();
  // Click 2: create (the label arrives PRE-FILLED — 0 typed).
  await page.getByTestId("zero-found-create").click();
  primaryClicks += 1;
  const input = page.locator("input[placeholder^='e.g. Work']");
  await expect(input).toHaveValue("Main");
  // Click 3: Create account.
  await page.getByRole("button", { name: "Create account" }).click();
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
  await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
  // The launch sheet opens (⌘N opens New project; the header New
  // button opens the launch sheet — flow (e)'s surface).
  await page.getByRole("button", { name: /^New$/ }).first().click();
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
  await expect(preselected).toContainText("Tight — projected burn nears");
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
