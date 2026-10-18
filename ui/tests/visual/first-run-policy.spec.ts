import { expect, test, type Page } from "@playwright/test";
import { honestEmpty, answerPolicy } from "./fixtures";

/**
 * AJ-E309 quality gate — the first-run policy hook end to end in the
 * production build: every option choosable, the Summary restates the
 * ACTIVE choice, "Decide later" writes nothing, the flow (a) budget
 * still holds (≤ 5 clicks with the policy answer), renewal dates are
 * optional with "expiry unknown" as the honest empty, [Expert setup…]
 * opens with its contents, and no funnel event carries a path, label,
 * or credential.
 */

const INSTALLS = [
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

const RECOMMENDATION = {
  items: [
    { provider: "claude-code", label: "Work", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-work" },
    { provider: "claude-code", label: "Side", action: "import", rationale: "Active — you'll sign in after import", default_on: true, path: "/Users/you/.claude-side" },
  ],
  summary: "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
  start_here: "/Users/you/dev-api",
};

const firstRun = async (page: Page, theme: "dark" | "light" = "dark") => {
  await page.addInitScript(
    ({ fixture, installs, recommendation, t }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = installs;
      w.__AIJ_RECOMMENDATION__ = recommendation;
      w.__AIJ_SCAN_REPORT__ = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };
      w.__AIJ_LOGIN_STATE__ = "signed_in";
      localStorage.setItem("aij.theme", t);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty, installs: INSTALLS, recommendation: RECOMMENDATION, t: theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
};

const seam = <T,>(page: Page, name: string) =>
  page.evaluate((key) => (window as unknown as Record<string, unknown>)[key], name) as Promise<T>;

for (const [value, restated] of [
  ["strip", "Packs: strip secrets (all Providers)"],
  ["keep", "Packs: keep secrets — packs stay on this Mac (all Providers)"],
  ["ask", "Packs: ask each time (all Providers)"],
  ["decide_later", "Packs: not chosen yet — your first switch asks before any pack is built"],
] as const) {
  test(`first run — choose "${value}" → Summary restates it`, async ({ page }) => {
    await firstRun(page);
    await page.getByTestId("install-continue").click();
    await page.getByTestId("use-recommended").click();
    const panel = page.getByTestId("policy-panel");
    await expect(panel).toBeVisible();
    // No silent default: nothing checked, Accept waits.
    await expect(panel.locator("input[type='radio']:checked")).toHaveCount(0);
    await expect(page.getByTestId("accept-setup")).toBeDisabled();
    await answerPolicy(page, value);
    await expect(page.getByTestId("accept-setup")).toBeEnabled();
    await page.getByTestId("accept-setup").click();
    await expect(page.getByTestId("summary-exits")).toBeVisible();
    await expect(page.getByTestId("policy-restatement")).toHaveText(restated);
    const writes = await seam<(string | null)[] | undefined>(page, "__AIJ_REDACTION_SET_GLOBAL__");
    // Written ONCE on the accept click — or never, for Decide later.
    expect(writes ?? []).toEqual(value === "decide_later" ? [] : [value]);
  });
}

test("flow (a) with the policy answer stays ≤ 5 clicks, 0 typed", async ({ page }) => {
  await firstRun(page);
  await page.evaluate(() => {
    const w = window as unknown as { __typed: number };
    w.__typed = 0;
    window.addEventListener("input", (event) => {
      if ((event.target as HTMLInputElement).type !== "radio") w.__typed += 1;
    });
  });
  let clicks = 0;
  await page.getByTestId("install-continue").click();
  clicks += 1;
  await page.getByTestId("use-recommended").click();
  clicks += 1;
  await answerPolicy(page, "strip");
  clicks += 1;
  await page.getByTestId("accept-setup").click();
  clicks += 1;
  await expect(page.getByTestId("summary-exits")).toBeVisible();
  await page.getByTestId("launch-first-session").click();
  clicks += 1;
  expect(clicks).toBeLessThanOrEqual(5);
  expect(await page.evaluate(() => (window as unknown as { __typed: number }).__typed)).toBe(0);
});

test("renewal date: optional per Confirm row — set is written, empty reads 'expiry unknown'", async ({ page }) => {
  await firstRun(page);
  await page.getByTestId("install-continue").click();
  await page.getByTestId("use-recommended").click();
  const rows = page.getByTestId("renewal-date");
  await expect(rows).toHaveCount(2);
  await expect(rows.first()).toContainText("Expiry unknown");
  await page.getByTestId("renewal-date-add").first().click();
  const input = page.getByLabel("Renewal date for Work (optional)");
  await expect(input).toHaveAttribute("type", "date");
  await input.fill("2026-11-30");
  await answerPolicy(page, "strip");
  await page.getByTestId("accept-setup").click();
  await expect(page.getByTestId("summary-exits")).toBeVisible();
  const restated = page.getByTestId("renewal-restatement");
  await expect(restated.nth(0)).toHaveText("Work: renews 2026-11-30 — we'll warn you before it");
  await expect(restated.nth(1)).toHaveText("Side: expiry unknown");
});

test("[Expert setup…] opens with folder, Reveal, shell line, env overrides, Import from folder", async ({ page }) => {
  await firstRun(page);
  await page.getByTestId("install-continue").click();
  await expect(page.getByTestId("use-recommended")).toBeVisible();
  const hatch = page.getByTestId("expert-hatch");
  await expect(hatch).toHaveAttribute("aria-expanded", "false");
  await hatch.click();
  await expect(hatch).toHaveAttribute("aria-expanded", "true");
  const panel = page.getByTestId("expert-panel");
  await expect(panel.getByTestId("expert-state-dir")).toHaveAttribute("readonly", "");
  await expect(panel.getByTestId("expert-reveal")).toBeVisible();
  await expect(panel.getByTestId("expert-snippet-pending")).toBeVisible();
  await expect(panel.getByTestId("expert-env-pending")).toBeVisible();
  await expect(panel.getByTestId("expert-import-folder")).toBeVisible();
  // After the accept, the Summary's hatch carries the real shell line.
  await page.getByTestId("use-recommended").click();
  await answerPolicy(page, "strip");
  await page.getByTestId("accept-setup").click();
  await expect(page.getByTestId("summary-exits")).toBeVisible();
  await page.getByTestId("expert-hatch").last().click();
  await expect(page.getByTestId("expert-snippet")).toContainText("CLAUDE_CONFIG_DIR=");
});

test("funnel events are local and carry no paths, labels, or credentials", async ({ page }) => {
  await firstRun(page);
  await page.getByTestId("install-continue").click();
  await page.getByTestId("use-recommended").click();
  await answerPolicy(page, "ask");
  await page.getByTestId("accept-setup").click();
  await expect(page.getByTestId("summary-exits")).toBeVisible();
  const events =
    (await seam<{ kind: string; fields: Record<string, string> }[]>(page, "__AIJ_FUNNEL_EVENTS__")) ?? [];
  expect(events.map((event) => event.kind)).toEqual(
    expect.arrayContaining(["flow_step", "proposal_shown", "policy_chosen", "proposal_accepted", "import_started"]),
  );
  for (const event of events) {
    for (const [key, fieldValue] of Object.entries(event.fields)) {
      expect(["path", "label", "token", "dir", "source", "account", "email", "key", "name", "secret"]).not.toContain(key);
      expect(fieldValue).toMatch(/^[a-z0-9][a-z0-9_:-]*$/);
    }
  }
  expect(JSON.stringify(events)).not.toMatch(/Users|Work|Side|claude-/);
});
