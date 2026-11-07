/**
 * 0.4 (D-62, AJ-E3207) gallery: New session, as the multi-subscription
 * user meets it. Opened from the top bar on a Mac with ten projects and
 * four Apps on three Accounts (one not signed in): the usual made and
 * said (N1, at 1280 and 1024), Change folder and Change app open (N2),
 * More options open (N3), a first run with no history (N4), and the
 * usual App's Account out of room (N5). Both themes, axe 0
 * critical/serious on each.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-new-session");
const T0 = new Date(2026, 10, 2, 15, 0, 0).getTime();
const MIN = 60_000;
const H = 60 * MIN;

const folder = (id: string, provider: string, label: string, login_state = "logged_in") => ({ id, provider, label, plan: null, login_state, cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [
  folder("claude-work", "claude-code", "Work Max"),
  folder("claude-personal", "claude-code", "Personal"),
  folder("codex-home", "codex", "~/.codex"),
  folder("claude-side", "claude-code", "Side project", "logged_out"),
];

const recipe = (id: string, account_id: string, account_label: string, provider: string, name: string, model: string | null) => ({ id, name, account_id, account_label, provider, model, default_cwd: null, restart_on_crash: null });
const RECIPES = [
  recipe("p-1", "claude-work", "Work Max", "claude-code", "Main", "opus"),
  recipe("p-2", "claude-work", "Work Max", "claude-code", "Quick fixes", "sonnet"),
  recipe("p-3", "claude-personal", "Personal", "claude-code", "Main", null),
  recipe("p-4", "codex-home", "~/.codex", "codex", "Main", null),
  recipe("p-5", "claude-side", "Side project", "claude-code", "Main", null),
];

const member = (id: string, label: string) => ({ id, label, folder: `~/.config/${id}`, plan: null, renewal_date: null, fee_usd: null });
const account = (key: string, tool: string, extra: Record<string, unknown>, profiles: ReturnType<typeof member>[]) => ({
  key,
  account_id: `acct:${key}`,
  name: null,
  tool,
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  plan: null,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  profiles,
  ...extra,
});
const SUBSCRIPTIONS = [
  account("work", "claude-code", { name: "Work", plan: "max-20x" }, [member("claude-work", "Work Max")]),
  account("personal", "claude-code", { plan: "pro", email: "y…@example.com" }, [member("claude-personal", "Personal")]),
  account("codex", "codex", { plan: "plus", email: "y…@example.com" }, [member("codex-home", "~/.codex")]),
  { ...account("out-claude", "claude-code", { state: "signed_out", account_id: null }, [member("claude-side", "Side project")]) },
];

const NAMES = ["web-shop", "payments-api", "ai-juggler", "landing-page", "data-pipeline", "mobile-app", "infra", "docs-site", "design-system", "scratch"];
const PROJECTS = NAMES.map((name, at) => ({
  id: `proj-${name}`,
  root: `/Users/you/Dev/${name}`,
  name,
  last_activity_ms: name === "scratch" ? null : T0 - (12 + at * 95) * MIN,
  active_instance: null,
  accounts_used: name === "web-shop" ? ["Personal", "Work Max"] : name === "payments-api" ? ["~/.codex"] : [],
  instance_count: name === "scratch" ? 0 : 3 + at,
  history_sessions: name === "scratch" ? 0 : 2,
  history_last_used_ms: null,
  repo: null,
  missing: false,
  pinned: name === "infra",
}));

const room = (account: string, left: number, resetsIn: number) => ({
  account,
  five_hour: { state: left === 0 ? "exhausted" : "ok", percentage: 100 - left, resets_at_ms: T0 + resetsIn, badge: "measured", measured_at_ms: T0 - 2 * MIN },
  weekly: null,
  forecast: { state: "learning", badge: "learning" },
  renewal: { state: "none", label: "", entered_by_user: false },
});
const ROOM = [room("claude-work", 58, 3 * H + 52 * MIN), room("claude-personal", 82, 2 * H), room("codex-home", 40, 4 * H)];
const OUT_OF_ROOM = [room("claude-work", 0, 1 * H + 20 * MIN), room("claude-personal", 82, 2 * H), room("codex-home", 40, 4 * H)];

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light", { projects = PROJECTS as unknown[], quota = ROOM as unknown[], width = 1280 } = {}) {
  await page.addInitScript(
    ({ fixture, recipes, quota, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_RECIPES__ = recipes;
      w.__AIJ_QUOTA_DISPLAY__ = quota;
      w.__AIJ_RANKING__ = [];
      w.__AIJ_ADVICE__ = [];
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: ACCOUNTS, projects, sessions: [], subscriptions: SUBSCRIPTIONS }, recipes: RECIPES, quota, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height: width === 1280 ? 800 : 700 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
  await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
  await expect(page.getByTestId("new-session-sheet")).toBeVisible();
}

const sentence = (page: Page) => page.getByTestId("new-session-sentence");

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${theme}.png`) });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]} ${finding.why}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`N1 the usual, made and said — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await expect(sentence(page)).toHaveText("Start in web-shop on Claude Code · Work Max — the last you used here");
    await expect(page.getByTestId("launch-runway")).toContainText("58% left");
    await expect(page.getByTestId("launch-now")).toBeFocused();
    await shoot(page, "usual", theme);
  });

  test(`N1 at 1024 — ${theme}`, async ({ page }) => {
    await open(page, theme, { width: 1024 });
    await expect(sentence(page)).toContainText("Start in web-shop");
    await shoot(page, "usual-1024", theme);
  });

  test(`N2 Change folder — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await page.getByTestId("change-folder").click();
    await expect(page.getByTestId("change-folder-search")).toBeFocused();
    await shoot(page, "change-folder", theme);
    await page.getByTestId("change-folder-search").fill("pay");
    await page.keyboard.press("Enter");
    await expect(sentence(page)).toHaveText("Start in payments-api on Codex · ~/.codex — the last you used here");
  });

  test(`N2 Change app — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await page.getByTestId("change-app").click();
    await expect(page.getByTestId("change-app-claude-side")).toContainText("Not signed in");
    await shoot(page, "change-app", theme);
    await page.getByTestId("change-app-claude-personal").click();
    await expect(sentence(page)).toHaveText("Start in web-shop on Claude Code · Personal");
  });

  test(`N3 More options — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await expect(page.getByTestId("new-session-more-values")).toHaveText("· Main · Opus");
    await page.getByTestId("new-session-more").locator("summary").click();
    await expect(page.getByTestId("runtime-pick")).toBeVisible();
    await shoot(page, "more-options", theme);
  });

  test(`N4 first time, no history — ${theme}`, async ({ page }) => {
    await open(page, theme, { projects: [] });
    await expect(sentence(page)).toHaveText("Start a session on Claude Code · Personal — the most room left");
    await shoot(page, "first-time", theme);
  });

  test(`N5 the usual is out of room — ${theme}`, async ({ page }) => {
    await open(page, theme, { quota: OUT_OF_ROOM });
    await expect(sentence(page)).toHaveText("Start in web-shop on Claude Code · Work Max — the last you used here");
    await expect(page.getByTestId("launch-out-of-room")).toContainText("Work is out of room until");
    await expect(page.getByTestId("launch-use-roomier")).toHaveText("Use Claude Code · Personal (82% left)");
    await shoot(page, "out-of-room", theme);
  });
}
