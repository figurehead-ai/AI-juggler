/**
 * AJ-E3013 gallery (D-116, D-119): desktop apps in a pool through AI
 * Juggler's connector. Fixtures only: the connector is never added,
 * removed or run here (the preview keeps its state in memory).
 *
 * - members: the canvas's column with a Claude app profile (an App
 *   member) Watched for want of the connector, with Add; a Claude app
 *   session connected ("Reads messages on its next turn"); another
 *   Watched with Add; a Codex app thread that can't take it or be woken.
 * - card: the App selected, its inspector's consent card.
 * - preview: Add's preview in one plain sentence, the files under "Show
 *   the files"; preview-text: the files shown, the manifest's text open.
 * - guide: "Show me how to add it by hand": the steps, the command and Copy,
 *   watched for while open.
 * - connected: the connected session selected: how a message reaches it,
 *   and its app's connector with Remove.
 * - new-pool: New pool's Who's in with the Claude app profile in it, the
 *   same consent card beside who's in.
 *
 * Both themes, 1280×800; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { goTo } from "./fixtures";
import { canvasSeed } from "./fixtures/pool-canvas";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-desktop-connector");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

const POOL = "pool-checkout";
const WORK = "app:claude-app:/Users/you/Profiles/Work";
const PERSONAL = "app:claude-app:/Users/you/Profiles/Personal";
const CODEX = "app:codex-app:";

/** The pool's members: the Work App (with `open` of its sessions open now)
 * and three desktop sessions named on their own. */
function poolRules(t0: number, open: number) {
  const rule = (member: string, kind: string, role: string, sessions = 0) => ({ member, kind, role, level: null, joined_at: t0 - 3_600_000, sessions, session_ids: [] });
  return {
    [POOL]: {
      members: [
        rule(`app:${WORK}`, "app", "member", open),
        rule("conversation:claude-personal/outside-4", "session", "reviewer"),
        rule("conversation:claude-work/outside-3", "session", "member"),
        rule("conversation:codex/outside-6", "session", "member"),
      ],
    },
  };
}

function seed(t0: number) {
  const base = canvasSeed(t0);
  const place = (id: string, app_id: string, app: string, instance: string | null, profile: string | null, label: string, account_id: string) => ({ id, app_id, app, instance, profile, label, account_id, app_pid: null, lost: false, keep_alive: false, paused: null, seen_ms: t0 - 3_600_000, cannot_start: null, recent: [], found: false });
  return {
    ...base,
    __AIJ_PLACES__: {
      places: [
        place(WORK, "claude-app", "Claude", "Work", "Work Max", "Claude · Work Max", "claude-work"),
        place(PERSONAL, "claude-app", "Claude", "Personal", "Personal", "Claude · Personal", "claude-personal"),
        place(CODEX, "codex-app", "ChatGPT", null, null, "ChatGPT", "codex"),
      ],
    },
    __AIJ_POOL_RULES__: poolRules(t0, 0),
    __AIJ_POOL_WATCHED__: {
      "conversation:claude-personal/outside-4": { title: "Plan the trip notes app", tool: "Claude app" },
      "conversation:claude-work/outside-3": { title: "Migrate the billing webhooks", tool: "Claude app" },
      "conversation:codex/outside-6": { title: "Landing page copy", tool: "ChatGPT" },
    },
    __AIJ_CONNECTOR__: { [PERSONAL]: { state: "current" } },
    __AIJ_DESKTOP_MEMBERS__: {
      [POOL]: [
        { member: `app:${WORK}`, app_id: WORK, connector: "absent", connected: false, wake: "next_turn" },
        { member: "conversation:claude-personal/outside-4", app_id: PERSONAL, connector: "current", connected: true, wake: "next_turn" },
        { member: "conversation:claude-work/outside-3", app_id: WORK, connector: "absent", connected: false, wake: "next_turn" },
        { member: "conversation:codex/outside-6", app_id: CODEX, connector: "unavailable", connected: false, wake: "cant_wake" },
      ],
    },
  };
}

// AJ-E3013 (f1b9bd354, 93b495f28): the Work App with its connector and
// two of its sessions in the pool through it; or a file in it changed.
function workConnector(state: "current" | "changed") {
  const changed = state === "changed" ? ["/Users/you/Profiles/Work/claude-code/skills/ai-juggler/skills/pool-tasks/SKILL.md"] : [];
  const session = (id: string, name: string, role: string, fresh: boolean) => ({ member: `conversation:claude-work/${id}`, session_id: fresh ? `desktop:claude-work/${id}` : null, name, place: "Claude app · Work Max", tool: "Claude Code", account: "Work Max", role, app_id: WORK, via: "app", wake: "next_turn" });
  return {
    __AIJ_CONNECTOR__: { [PERSONAL]: { state: "current" }, [WORK]: { state, changed } },
    __AIJ_DESKTOP_MEMBERS__: {
      [POOL]: [
        { member: `app:${WORK}`, app_id: WORK, connector: state, connected: state === "current", wake: "next_turn" },
        { member: "conversation:claude-personal/outside-4", app_id: PERSONAL, connector: "current", connected: true, wake: "next_turn" },
        { member: "conversation:claude-work/outside-3", app_id: WORK, connector: state, connected: false, wake: "next_turn" },
        { member: "conversation:codex/outside-6", app_id: CODEX, connector: "unavailable", connected: false, wake: "cant_wake" },
      ],
    },
    __AIJ_DESKTOP_SESSIONS__: { [POOL]: state === "current" ? [session("outside-7", "Tidy the checkout tests", "member", true), session("outside-8", "Fix the coupon rounding", "reviewer", false)] : [] },
    // The backend counts an attested session among its App's open ones.
    __AIJ_POOL_RULES__: poolRules(GATE_NOW, state === "current" ? 1 : 0),
  };
}

// Round 7 #7 (aij-test run 3): Planner and the Work App's desktop session
// messaged each other; the diagram draws their lines from its row.
function desktopLines(t0: number) {
  const base = canvasSeed(t0).__AIJ_POOL_CANVAS__[POOL]!;
  const line = (from: string, to: string, messages: number, ago: number) => ({ from_session_id: from, to_session_id: to, messages, tasks: 0, last_at_ms: t0 - ago, thread_id: "thr-copy", pending: [] });
  const desktop = "conversation:claude-work/outside-7";
  return { __AIJ_POOL_CANVAS__: { [POOL]: { ...base, edges: [...base.edges, line("s-plan", desktop, 2, 240_000), line(desktop, "s-plan", 1, 120_000)] } } };
}

async function open(page: Page, theme: "dark" | "light", extra: Record<string, unknown> = {}) {
  await page.clock.setFixedTime(GATE_NOW);
  await page.addInitScript(
    ({ theme, seed }) => {
      Object.assign(window, seed);
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, seed: { ...seed(GATE_NOW), ...extra } },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function shoot(page: Page, name: string) {
  await page.waitForTimeout(150);
  await page.screenshot({ path: resolve(OUT, name), animations: "disabled" });
}

async function poolPage(page: Page) {
  await goTo(page, "Pools");
  await page.getByTestId("pool-card-tile").filter({ hasText: "Checkout team" }).click();
  await expect(page.getByTestId("pool-canvas")).toBeVisible();
  await expect(page.getByTestId("canvas-watched-node")).toHaveCount(3);
}

for (const theme of ["dark", "light"] as const) {
  test(`desktop members and the consent card, its preview and guide — ${theme}`, gate(`99-desktop-connector-${theme}`), async ({ page }) => {
    await open(page, theme);
    await poolPage(page);
    const reach = page.getByTestId("canvas-watched-reach");
    await expect(reach.nth(0)).toHaveText("Claude app · Reads messages on its next turn");
    await expect(reach.nth(1)).toHaveText("Claude app · Its app has no AI Juggler connector");
    await expect(reach.nth(2)).toHaveText("ChatGPT · Its app can't take AI Juggler's connector yet; it can't be woken");
    await expect(page.locator(`[data-rule-ref="app:${WORK}"]`).getByTestId("canvas-rule-wake")).toHaveText("Watched: its app has no AI Juggler connector");
    await axe(page, `members ${theme}`);
    await shoot(page, `members-1280x800-${theme}.png`);

    // The App's Add opens it on the consent card.
    await page.getByTestId("canvas-rule-connector-add").click();
    const card = page.getByTestId("inspect-connector");
    await expect(card.locator("h4")).toHaveText("Add AI Juggler's connector to Claude app · Work Max so its sessions can talk in this pool");
    await axe(page, `card ${theme}`);
    await shoot(page, `card-1280x800-${theme}.png`);

    await page.getByTestId("inspect-connector-add").click();
    // Casey's PC1 walk: one plain sentence first, the files under a fold.
    await expect(page.getByTestId("inspect-connector-preview-sentence")).toHaveText("This adds a small helper to your Claude app (Work Max) so its sessions can talk in this pool: 2 new files in its own settings folder; nothing of yours changes.");
    await expect(page.getByTestId("inspect-connector-confirm")).toHaveText("Add to Claude app · Work Max");
    await axe(page, `preview ${theme}`);
    await shoot(page, `preview-1280x800-${theme}.png`);
    // Show the files: each file, and its exact text in its own fold.
    await page.getByTestId("inspect-connector-files-fold-toggle").click();
    await expect(page.getByTestId("inspect-connector-file")).toHaveCount(2);
    await page.getByTestId("inspect-connector-file-show-toggle").first().click();
    await expect(page.getByTestId("inspect-connector-file-text").first()).toContainText('"mcpServers"');
    await page.getByTestId("inspect-connector-file-show-toggle").first().scrollIntoViewIfNeeded();
    await shoot(page, `preview-text-1280x800-${theme}.png`);

    await page.getByTestId("inspect-connector-cancel").click();
    await page.getByTestId("inspect-connector-guide-open").click();
    await expect(page.getByTestId("turn-on-command")).toHaveText("/plugin marketplace add figurehead-ai/AI-juggler");
    await expect(page.getByTestId("inspect-connector-guide-watching")).toBeVisible();
    await axe(page, `guide ${theme}`);
    await shoot(page, `guide-1280x800-${theme}.png`);
  });

  test(`a connected desktop member, and New pool's consent card — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await poolPage(page);
    await page.getByTestId("canvas-watched-node").nth(0).getByRole("button", { name: /Plan the trip notes app/ }).click();
    // D-119 (5): a person's Deliver wakes it whatever the pool's level.
    await expect(page.getByTestId("inspect-rule-reach")).toHaveText("It talks in this pool through AI Juggler's connector. Reads messages on its next turn. A message you send with Deliver wakes it, whatever the pool's level.");
    await expect(page.getByTestId("inspect-connector-there")).toContainText("AI Juggler's connector is in this app.");
    await axe(page, `connected ${theme}`);
    await shoot(page, `connected-1280x800-${theme}.png`);

    await goTo(page, "Pools");
    await page.getByTestId("pools-new").click();
    for (let n = 0; n < 4 && !(await page.getByTestId("new-pool-step-who").isVisible()); n += 1) await page.getByTestId("new-pool-next").click();
    await page.getByRole("radiogroup", { name: "Kind of member" }).getByRole("radio", { name: /^Apps/ }).click();
    await page.getByTestId("new-pool-candidate").filter({ hasText: "Claude app · Work Max" }).getByRole("checkbox").click();
    // Round 7 (Casey's worst moment): the App's own row says it can't talk
    // yet, with Add, before Next.
    const row = page.getByTestId("new-pool-candidate").filter({ hasText: "Claude app · Work Max" });
    const state = row.getByTestId("new-pool-connector-card");
    await expect(state).toHaveAttribute("data-state", "absent");
    await expect(state).toContainText("Watched: can't talk in the pool yet.");
    await expect(row.getByTestId("new-pool-connector-card-add")).toBeVisible();
    await axe(page, `new pool ${theme}`);
    await shoot(page, `new-pool-1280x800-${theme}.png`);

    // Add: the consent card's own preview in the row, then added.
    await row.getByTestId("new-pool-connector-card-add").click();
    await expect(page.getByTestId("new-pool-connector-card-preview-sentence")).toContainText("This adds a small helper to your Claude app (Work Max)");
    await page.getByTestId("new-pool-connector-card-confirm").click();
    await expect(page.getByTestId("new-pool-connector-card-added")).toBeVisible();
    await axe(page, `new pool added ${theme}`);
    await shoot(page, `new-pool-added-1280x800-${theme}.png`);
  });

  test(`desktop sessions under their App, and each a recipient in Messages — ${theme}`, async ({ page }) => {
    await open(page, theme, workConnector("current"));
    await poolPage(page);
    const sessions = page.getByTestId("canvas-desktop-session");
    await expect(sessions).toHaveCount(2);
    await expect(sessions.nth(0)).toHaveText("Active lately: Tidy the checkout testsMember · reads messages on its next turn");
    await expect(page.locator(`[data-rule-ref="app:${WORK}"]`).getByTestId("canvas-rule-open")).toHaveText("1 open now · new ones in any project join");
    await expect(sessions.nth(1)).toHaveAttribute("data-member", "conversation:claude-work/outside-8");
    await axe(page, `sessions ${theme}`);
    await shoot(page, `sessions-1280x800-${theme}.png`);

    await page.getByTestId("pool-panel-messages").click();
    await page.getByTestId("messages-new").click();
    const form = page.getByRole("form", { name: "New message" });
    await form.getByTestId("compose-to").click();
    await expect(page.getByTestId("compose-to-conversation:claude-work/outside-7")).toHaveText(/Tidy the checkout tests · Claude app · Work Max/);
    await expect(page.getByTestId("compose-to-conversation:claude-work/outside-8")).toBeVisible();
    await axe(page, `recipients ${theme}`);
    await shoot(page, `recipients-1280x800-${theme}.png`);
  });

  test(`Remove refused by a changed file: Remove anyway, Cancel focused — ${theme}`, async ({ page }) => {
    await open(page, theme, workConnector("changed"));
    await poolPage(page);
    await expect(page.getByTestId("canvas-rule-connector-add")).toHaveText("Fix");
    await page.locator(`[data-rule-ref="app:${WORK}"]`).click();
    await expect(page.getByTestId("inspect-connector-changed")).toContainText("SKILL.md");
    await page.getByTestId("inspect-connector-remove").click();
    // Round 7 #8: nothing was taken out, said plainly; then what Remove
    // anyway would delete, never "Deletes …" as if Remove will.
    await expect(page.getByTestId("inspect-connector").locator("h4")).toHaveText("Nothing was taken out of Claude app · Work Max");
    await expect(page.getByTestId("inspect-connector-remove-changed")).toHaveText("SKILL.md changed since AI Juggler added it, so Remove leaves the connector as it is.");
    await expect(page.getByTestId("inspect-connector")).toContainText("Remove anyway would delete:");
    await expect(page.getByTestId("inspect-connector")).not.toContainText("Deletes ");
    await expect(page.getByTestId("inspect-connector-remove-anyway")).toHaveText("Remove anyway (also deletes SKILL.md)");
    await expect(page.getByTestId("inspect-connector-cancel")).toBeFocused();
    // Scrolled to the step: the app's edge under the inspector's head.
    await expect(page.getByTestId("inspector-body")).toHaveAttribute("data-scrolled", "");
    await expect(page.getByTestId("inspect-connector-remove-anyway")).toBeInViewport();
    await axe(page, `remove anyway ${theme}`);
    await shoot(page, `remove-anyway-1280x800-${theme}.png`);
  });

  test(`a line between a board node and a desktop session: its count, selected, its own rule — ${theme}`, async ({ page }) => {
    await open(page, theme, { ...workConnector("current"), ...desktopLines(GATE_NOW) });
    await poolPage(page);
    const lines = page.getByTestId("canvas-desktop-line");
    await expect(lines).toHaveCount(2);
    const toIt = page.locator('[data-testid="canvas-desktop-line"][data-edge="s-plan>conversation:claude-work/outside-7"]');
    await expect(toIt).toHaveText("2 messages");
    await axe(page, `desktop line ${theme}`);
    await shoot(page, `desktop-line-1280x800-${theme}.png`);

    await toIt.click();
    await expect(page.getByTestId("inspector-title")).toHaveText("Planner → Tidy the checkout tests");
    await page.getByTestId("edge-talk-rule").getByRole("radio", { name: "Ask first" }).click();
    await expect(toIt).toHaveText("2 messages · Ask first here");
    await axe(page, `desktop line selected ${theme}`);
    await shoot(page, `desktop-line-selected-1280x800-${theme}.png`);
  });

  // Round 7 #9 (aij-test run 3): "Add a session" says how a desktop app's
  // sessions take part once for its list; each row only where it stands.
  test(`Add a session: the desktop note once, each row Watched or Can talk — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await poolPage(page);
    const palette = page.getByTestId("canvas-palette");
    if (!(await page.getByTestId("canvas-palette-body").isVisible())) await palette.getByRole("button", { name: /Add a member/ }).click();
    await page.getByTestId("canvas-palette-kind").click();
    await page.getByRole("menuitemradio", { name: /^Apps/ }).click();
    await expect(page.getByTestId("canvas-palette-more-note")).toHaveCount(1);
    await expect(page.getByTestId("canvas-palette-more-note")).toContainText("Sessions in a desktop app talk in the pool once AI Juggler's connector");
    const personal = page.getByTestId("canvas-palette-more-item").filter({ hasText: "Claude app · Personal" });
    await expect(personal.getByTestId("canvas-palette-more-reach")).toHaveText("Can talk");
    await expect(page.getByTestId("canvas-palette-more-item").filter({ hasText: "a small helper" })).toHaveCount(0);
    await axe(page, `add a session ${theme}`);
    await shoot(page, `add-a-session-1280x800-${theme}.png`);
  });
}
