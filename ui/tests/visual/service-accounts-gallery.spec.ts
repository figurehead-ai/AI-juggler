/**
 * AJ-E1601 gallery: accounts through another service (OpenCode Go, Kimi).
 * Add account's chooser with More open, the service's sheet (its key in a
 * masked field), the Accounts page with an OpenCode Go card (estimate)
 * over a Claude Code and a Codex profile beside a Kimi card (usage
 * unknown), and a profile whose key isn't on this Mac. 1280×800, both
 * themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/94-service-accounts");
const T0 = 1_728_800_000_000;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

const INSTALLS = [
  { provider: "claude-code", display_name: "Claude Code", install_url: "https://claude.com/download", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [], verified_against: ["2.1.x"], kind: "terminal", single_instance_note: null },
  { provider: "codex", display_name: "Codex", install_url: "https://developers.openai.com/codex", status: "installed", version: "0.163.0", binary: "/opt/homebrew/bin/codex", notes: [], verified_against: ["0.163.0"], kind: "terminal", single_instance_note: null },
];

const profile = (id: string, provider: string, label: string, service: string, plan: string, signedIn = true) => ({
  id,
  provider,
  label,
  plan,
  login_state: signedIn ? "signed_in" : "logged_out",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: false,
  service,
});

const GO = {
  id: "opencode-go",
  name: "OpenCode Go",
  short_name: "Go",
  usage: "estimate",
  usage_words:
    "Its limits are dollars per model: every 5 hours allows a fifth of the month's amount, and every week half. OpenCode doesn't tell other tools what's left, so AI Juggler can only estimate it from what its sessions used.",
  console_url: "https://opencode.ai/go",
  key_name: "OpenCode Go API key",
};
const KIMI = {
  id: "kimi",
  name: "Kimi",
  short_name: "Kimi",
  usage: "unknown",
  usage_words: "Kimi counts a 5-hour window and a monthly total, shared with its own apps. It doesn't tell other tools what's left, so the limits are unknown here; Kimi's own pages show them.",
  console_url: "https://www.kimi.com/code/docs/en/kimi-code/membership.html",
  key_name: "Kimi Code API key",
};

const member = (id: string, plan: string) => ({ id, folder: `~/Library/Application Support/AI Juggler/accounts/${id}`, plan, renewal_date: null, fee_usd: null });
const subscription = (key: string, ids: string[], plan: string, service: unknown, extra: Record<string, unknown> = {}) => ({
  key,
  tool: "claude-code",
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: ids.map((id) => member(id, plan)),
  plan,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  service,
  ...extra,
});

/** OpenCode Go on both tools, Kimi on Claude Code; `missing`: Kimi's key
 * isn't on this Mac (a restored backup). */
function fixture(missing: boolean) {
  return {
    accounts: [
      profile("go", "claude-code", "Go", "OpenCode Go", "Go Plus"),
      profile("go-codex", "codex", "Go Codex", "OpenCode Go", "Go Plus"),
      profile("kimi", "claude-code", "Kimi", "Kimi", "Pro", !missing),
    ],
    profiles: [],
    sessions: [],
    projects: [],
    conversations: [],
    subscriptions: [
      subscription("go", ["go", "go-codex"], "Go Plus", GO),
      subscription("kimi", ["kimi"], "Pro", KIMI, missing ? { state: "unverified", reason: "its Kimi Code API key isn't in this Mac's Keychain" } : {}),
    ],
  };
}

async function seed(page: Page, theme: "dark" | "light", data: unknown) {
  await page.addInitScript(
    ({ data, theme, t0, installs, burned }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = data;
      w.__AIJ_PROVIDER_INSTALLS__ = installs;
      w.__AIJ_QUOTA_DISPLAY__ = [
        {
          account: "go",
          five_hour: { state: "unmeasured", percentage: null, resets_at_ms: null, burned_tokens: burned.five },
          weekly: { state: "unmeasured", percentage: null, resets_at_ms: null, burned_tokens: burned.week },
          renewal: { state: "none", label: "", entered_by_user: false },
          forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
        },
      ];
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { data, theme, t0: T0, installs: INSTALLS, burned: { five: 420_000, week: 3_100_000 } },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-1280x800-${theme}.png`), fullPage: true });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`Add account › More › Through another service — ${theme}`, gate("94-service-accounts"), async ({ page }) => {
    await seed(page, theme, fixture(false));
    await goTo(page, "Accounts");
    await page.getByTestId("add-account").click();
    await page.getByTestId("add-more").locator("summary").click();
    await expect(page.getByTestId("service-card")).toHaveCount(2);
    await shoot(page, "add-account-more", theme);
  });

  test(`the service's sheet: plan, key (masked) and tools — ${theme}`, async ({ page }) => {
    await seed(page, theme, fixture(false));
    await goTo(page, "Accounts");
    await page.getByTestId("add-account").click();
    await page.getByTestId("add-more").locator("summary").click();
    await page.getByTestId("service-card").filter({ hasText: "OpenCode Go" }).getByTestId("service-choose").click();
    await page.getByTestId("service-key").fill("fixture-key-for-the-gallery-0000");
    await page.getByTestId("service-tool").filter({ hasText: "Codex" }).getByTestId("service-tool-on").check();
    await expect(page.getByTestId("service-key")).toHaveAttribute("type", "password");
    await expect(page.getByTestId("service-add")).toBeDisabled(); // Codex's model is still to type
    await page.getByTestId("service-tool").filter({ hasText: "Codex" }).getByTestId("service-model").fill("a-model");
    await expect(page.getByTestId("service-add")).toBeEnabled();
    await shoot(page, "service-sheet", theme);
  });

  test(`Accounts: OpenCode Go (estimate) and Kimi (unknown) — ${theme}`, async ({ page }) => {
    await seed(page, theme, fixture(false));
    await goTo(page, "Accounts");
    const go = page.locator("[data-testid='account-card'][data-service='opencode-go']");
    await expect(go.getByTestId("account-line")).toHaveText("Go Plus · estimate");
    await expect(go.getByTestId("profile-row")).toHaveCount(2);
    await expect(go.getByTestId("account-service-used")).toContainText("420k tokens in the last 5 hours");
    const kimi = page.locator("[data-testid='account-card'][data-service='kimi']");
    await expect(kimi.getByTestId("account-line")).toHaveText("Pro · usage unknown");
    await shoot(page, "accounts-service", theme);
  });

  test(`a profile whose key isn't on this Mac — ${theme}`, async ({ page }) => {
    await seed(page, theme, fixture(true));
    await goTo(page, "Accounts");
    const kimi = page.locator("[data-testid='account-card'][data-service='kimi']");
    await expect(kimi.getByTestId("profile-key-missing")).toBeVisible();
    await expect(kimi.getByTestId("account-new-key")).toHaveText("Add its key…");
    await shoot(page, "accounts-key-missing", theme);
  });
}
