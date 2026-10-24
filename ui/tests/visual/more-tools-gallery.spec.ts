/**
 * E16 gallery: the tools that joined Claude Code and Codex, where tools
 * appear — Add account's tool chooser, first run with their setups
 * found, and each tool's own page (More › Tools). Every state is the
 * same screen the existing tools use, fed by the tools catalog
 * (`ui/src/preview/tools.ts`, mirroring the adapters' specs). 1280×800,
 * both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, honestEmpty } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/91-more-tools");
const THEMES = ["dark", "light"] as const;
// A pinned clock (AJ-E1522): "Used 3 days ago" reads the same every day.
const NOW_MS = 1_728_800_000_000;
const now = () => Math.floor(NOW_MS / 1000);

/** The E16 tools: id, name, version found, and a default folder. */
const NEW_TOOLS = [
  { id: "opencode", name: "OpenCode", version: "1.19.3", binary: "/opt/homebrew/bin/opencode", folder: "/Users/you/.local/share/opencode" },
] as const;

const INSTALLS = [
  { provider: "claude-code", display_name: "Claude Code", install_url: "https://claude.com/download", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [], verified_against: ["2.1.x"], kind: "terminal", single_instance_note: null },
  ...NEW_TOOLS.map((tool) => ({
    provider: tool.id,
    display_name: tool.name,
    install_url: "",
    status: "unverified",
    version: tool.version,
    binary: tool.binary,
    notes: [`${tool.name} — built from its source; not yet checked live`],
    verified_against: [],
    kind: "terminal",
    single_instance_note: null,
  })),
];

const found = (harness: string, path: string, days: number) => ({
  harness,
  path,
  origin: "vendor-default",
  login: "logged_in",
  bytes: 30_000_000,
  last_used_secs: now() - days * 86_400,
  managed: false,
  notes: [],
  inference: {
    purpose: "Main",
    purpose_confidence: "likely",
    login: "signed-in",
    login_confidence: "found",
    activity: "active",
    activity_words: "Active this week",
    activity_confidence: "found",
    keychain_bound: false,
  },
  signals: ["vendor-default"],
});

const SCAN = {
  installs: [found("claude-code", "/Users/you/.claude", 2), ...NEW_TOOLS.map((tool) => found(tool.id, tool.folder, 1))],
  probes: [],
  seeds: [],
  desktop_apps: [],
  cancelled: false,
};

/** One account on each new tool, for its page under More › Tools. */
const WITH_ACCOUNTS = {
  ...honestEmpty,
  accounts: NEW_TOOLS.map((tool, n) => ({ id: `new-${n}`, provider: tool.id, label: tool.name, plan: null, login_state: "logged_in" })),
  stats: { accounts: NEW_TOOLS.length, profiles: 0, active_sessions: 0, ledger_degraded: false },
};

/** What the backend serves for them: each can use its default folder
 * where it is; none reports its limits. */
const CAPABILITIES = [
  { provider: "claude-code", can_use_in_place: true, reports_limits: true },
  { provider: "codex", can_use_in_place: true, reports_limits: true },
  ...NEW_TOOLS.map((tool) => ({ provider: tool.id, can_use_in_place: true, reports_limits: false })),
];
const QUOTA = NEW_TOOLS.map((tool, n) => ({
  account: `new-${n}`,
  five_hour: null,
  five_hour_omitted_reason: null,
  weekly: null,
  weekly_omitted_reason: null,
  renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
  unavailable_reason: `${tool.name} doesn't report its limits to AI Juggler yet`,
}));

async function seed(page: Page, theme: string, fixture: unknown, scan: unknown) {
  await page.clock.setFixedTime(NOW_MS);
  await page.addInitScript(
    ({ fixture: f, scan: s, installs, capabilities, quota, t }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = f;
      w.__AIJ_PROVIDER_INSTALLS__ = installs;
      w.__AIJ_PROVIDER_CAPABILITIES__ = capabilities;
      w.__AIJ_QUOTA_DISPLAY__ = quota;
      localStorage.setItem("aij.theme", t);
      // A scan to show: first run, as on a fresh install.
      if (s) {
        w.__AIJ_SCAN_REPORT__ = s;
      } else {
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      }
    },
    { fixture, scan, installs: INSTALLS, capabilities: CAPABILITIES, quota: QUOTA, t: theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: 1280, height: 800 });
}

async function shoot(page: Page, name: string, theme: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
  mkdirSync(OUT, { recursive: true });
  await page.screenshot({ path: resolve(OUT, `${name}-1280x800-${theme}.png`), fullPage: true });
}

for (const theme of THEMES) {
  test(`Add account lists the new tools — ${theme}`, gate("91-more-tools"), async ({ page }) => {
    await seed(page, theme, honestEmpty, null);
    await page.goto("/");
    await goTo(page, "Accounts");
    await page.getByTestId("add-account").click();
    const cards = page.getByTestId("provider-cards");
    for (const tool of NEW_TOOLS) await expect(cards).toContainText(tool.name);
    await shoot(page, "add-account-chooser", theme);
  });

  test(`first run finds the new tools' setups, each in its own group — ${theme}`, async ({ page }) => {
    await seed(page, theme, { accounts: [], profiles: [], sessions: [], projects: [] }, SCAN);
    await page.goto("/");
    for (const tool of NEW_TOOLS) {
      const group = page.getByTestId(`first-run-group-${tool.id}`);
      await expect(group).toContainText(tool.name);
      await expect(group).toContainText(tool.folder.replace("/Users/you", "~"));
    }
    await shoot(page, "first-run-found", theme);
  });

  for (const tool of NEW_TOOLS) {
    test(`${tool.name}'s page, from More › Tools — ${theme}`, async ({ page }) => {
      await seed(page, theme, WITH_ACCOUNTS, null);
      await page.goto("/");
      await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "More", exact: true }).click();
      await page.getByTestId("more-tool").filter({ hasText: tool.name }).click();
      await expect(page.getByTestId("page-type")).toHaveText("Tool");
      await expect(page.getByRole("heading", { level: 1, name: tool.name })).toBeVisible();
      await shoot(page, `${tool.id}-tool-page`, theme);
    });
  }
}
