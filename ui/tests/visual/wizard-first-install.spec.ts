import { expect, test } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { answerPolicy, honestEmpty } from "./fixtures";

/**
 * AJ-E305 quality gate — flow (a) as a TESTED CONTRACT: the primary
 * path runs in ≤ 5 PRIMARY-button clicks with 0 typed fields, and
 * the funnel seam (`aij:funnel`) records every step with no
 * missing-step gaps (gate → scan → confirm → sign-ins → summary).
 *
 * The click count is asserted two ways: the spec's own .click()
 * calls are counted (each one a primary-button activation — skips
 * are secondary and counted separately), and every `input` event on
 * the page is recorded to prove nothing was typed.
 */

const GALLERY = resolve(process.cwd(), "../docs/qa/evidence/_gallery");

test.beforeAll(() => {
  mkdirSync(resolve(GALLERY, "37-flows"), { recursive: true });
});

test("flow (a): first install ≤ 5 primary clicks, 0 typed, funnel complete", async ({
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
        seeds: ["/Users/you/dev-api"],
        desktop_apps: [],
        cancelled: false,
      };
      w.__AIJ_RECOMMENDATION__ = {
        items: [
          {
            provider: "claude-code",
            label: "Work",
            action: "import",
            rationale: "Signed in — import keeps history",
            default_on: true,
            path: "/Users/you/.claude-work",
          },
          {
            provider: "claude-code",
            label: "Side",
            action: "import",
            rationale: "Active — you'll sign in after import",
            default_on: true,
            path: "/Users/you/.claude-side",
          },
        ],
        summary:
          "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
        start_here: "/Users/you/dev-api",
      };
      localStorage.setItem("aij.theme", "dark");
      sessionStorage.setItem("aij.ready.skipped", "1");
      // The funnel log + the typed-input recorder (both test-only).
      const log: string[] = [];
      w.__funnelLog = log;
      window.addEventListener("aij:funnel", (event) => {
        const detail = (event as CustomEvent<{ flow: string; step: string }>).detail;
        log.push(`${detail.flow}:${detail.step}`);
      });
      const typed: string[] = [];
      w.__typedLog = typed;
      window.addEventListener("input", (event) => {
        const target = event.target as HTMLInputElement;
        typed.push(`${target.tagName}:${target.type ?? ""}`);
      });
    },
    { fixture: honestEmpty },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize({ width: 1280, height: 800 });
  // The FIRST-RUN wizard (unlocked — the Accounts entry is flow (d),
  // locked, since AJ-E306): it opens itself on a clean first run.
  await page.goto("/");
  await expect(page.getByTestId("onboarding-wizard")).toBeVisible();

  let primaryClicks = 0;
  let secondaryClicks = 0;

  // Click 1 (primary): the Step-0 gate.
  await page.getByTestId("install-continue").click();
  primaryClicks += 1;
  await expect(page.getByTestId("proposal-teaser")).toBeVisible();

  // Click 2 (primary): Use recommended setup →.
  await page.getByTestId("use-recommended").click();
  primaryClicks += 1;
  await expect(page.getByTestId("proposal-block")).toBeVisible();

  // AJ-E309: the first-run pack-policy question rides this Confirm
  // (no extra step, nothing pre-selected) — one deliberate answer.
  let policyClicks = 0;
  await expect(page.getByTestId("accept-setup")).toBeDisabled();
  await answerPolicy(page, "strip");
  policyClicks += 1;

  // Click 3 (primary): Accept setup →.
  await page.getByTestId("accept-setup").click();
  primaryClicks += 1;

  // Both imports succeed but need the vendor login → the queue (2
  // cards, one at a time). [Skip for now] is a secondary control.
  await expect(page.getByTestId("signin-queue")).toBeVisible();
  await expect(page.getByTestId("signin-queue")).toContainText("1 of 2");
  for (let card = 0; card < 2; card += 1) {
    await page.getByTestId("signin-skip").click();
    secondaryClicks += 1;
  }
  await expect(page.getByTestId("summary-exits")).toBeVisible();

  // Click 4 (primary): Launch first session → (carries Start here).
  await page.getByTestId("launch-first-session").click();
  primaryClicks += 1;

  // The budget holds WITH the policy answer (AJ-E309 Req.1: it counts
  // inside the ≤ 5); skips are secondaries, not path clicks.
  expect(primaryClicks + policyClicks).toBeLessThanOrEqual(5);
  expect(primaryClicks).toBe(4);

  // 0 typed fields: the only input event is the policy RADIO (a
  // choice, not typing).
  const typed = (await page.evaluate(() =>
    (window as unknown as { __typedLog: string[] }).__typedLog,
  )) as string[];
  expect(typed.filter((entry) => entry !== "INPUT:radio")).toHaveLength(0);

  // The funnel recorded every step, in order, no gaps.
  const funnel = (await page.evaluate(() =>
    (window as unknown as { __funnelLog: string[] }).__funnelLog,
  )) as string[];
  const expected = [
    "a-first-install:gate",
    "a-first-install:scan",
    "a-first-install:confirm",
    "a-first-install:signins",
    "a-first-install:summary",
  ];
  for (const step of expected) {
    expect(funnel).toContain(step);
  }
  const indexes = expected.map((step) => funnel.indexOf(step));
  expect(indexes).toEqual([...indexes].sort((a, b) => a - b));
  expect(indexes[0]).toBeGreaterThanOrEqual(0);

  // The launch sheet opened with the Start-here project preselected.
  await expect(page.getByRole("heading", { name: "New session" })).toBeVisible();
  await expect(page.locator("text=dev-api").first()).toBeVisible();
});
