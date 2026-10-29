/**
 * AJ-E2501..E2507 gallery (the approved design of 2026-10-27,
 * docs/design/notifications-and-user-menu-2026-10-27.html): the header's
 * bell with its one count; the notification centre populated, empty and
 * muted (the quiet hours), and a need's ⋯; the alert banners (an account
 * almost out of room; AI Juggler's own service not answering); the toasts
 * (a result with Undo; an error with Retry, staying until closed); your
 * menu without and with a Figurehead account; and the menu-bar item's
 * menu, drawn from the Rust builder's own output
 * (`tests/visual/fixtures/tray-menu.json`, kept by the builder's test).
 * 1280×800, both themes; axe 0 critical/serious on every state.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/96-notification-centre");
const TRAY = resolve(process.cwd(), "tests/visual/fixtures/tray-menu.json");
const T0 = 1_728_800_000_000;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

const facts = { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null };
const target = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
const notice = (over: Record<string, unknown>) => ({ id: 1, kind: "finished", needs: false, title: "", detail: "", project_id: null, project: null, target, facts, action: null, meter: null, created_ms: T0, unread: false, snoozed_until_ms: null, muted: false, dismissed: false, ...over });

/** What the model holds: an account almost out of room (AI Juggler's own
 * need) and what happened today. Home's list adds the rest. */
const POPULATED = {
  app: [notice({ id: 41, kind: "quota", needs: true, unread: true, title: "Work Max is almost out of room", detail: "7% of its 5-hour window left", meter: 7, created_ms: T0 - 3 * 60_000, action: "hand-over", target: { ...target, account_id: "claude-work" }, facts: { ...facts, window: "5-hour", resets_at_ms: T0 + 95 * 60_000, sessions: 2, account: "Work Max" } })],
  earlier: [
    notice({ id: 40, kind: "reset", title: "Personal has room again — its week reset", detail: "Sessions on it can carry on", created_ms: T0 - 2 * 3_600_000, action: "open", target: { ...target, account_id: "claude-personal" } }),
    notice({ id: 39, kind: "restarted", title: "“Nightly fix” crashed and was restarted (try 1 of 3)", detail: "nightly-fix · Work Max", created_ms: T0 - 4 * 3_600_000, action: "see-why", target: { ...target, session_id: "sess-1" } }),
    notice({ id: 38, kind: "update", unread: true, title: "AI Juggler 0.2.1 is ready", detail: "Restart when you're ready — nothing restarts by itself", created_ms: T0 - 5 * 3_600_000, action: "whats-new", target: { ...target, page: "updates" }, facts: { ...facts, version: "0.2.1", channel: "beta" } }),
  ],
};

const LINKED = { state: "linked", email: "sam@example.com", plan: "Free", device_name: "Studio", features: [], link: null, refreshed_at_ms: T0, offline: false, notice: null, consent_version: 1, consent_current: 1 };
const PEER = { machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA", permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code", paired_at_ms: 1, last_seen_ms: null, connection: "link_off", protocol: null, connection_detail: null };

interface State {
  theme: "dark" | "light";
  notices?: unknown;
  offline?: boolean;
  fail?: string;
  account?: boolean;
  /** Nothing waits: only running sessions. */
  calm?: boolean;
}

const CALM = { ...allStatuses, sessions: allStatuses.sessions.filter((s) => s.status === "running" || s.status === "starting") };

/** One init script for the whole test; each state is read from this
 * tab's session storage, so a reload switches it. */
async function prepare(page: Page): Promise<void> {
  await page.addInitScript(({ t0, fixture, calm, linked, peers }) => {
    const state = JSON.parse(sessionStorage.getItem("aij.gallery.state") ?? "{}") as { theme?: string; notices?: unknown; offline?: boolean; fail?: string; account?: boolean; calm?: boolean };
    const w = window as unknown as Record<string, unknown>;
    w.__AIJ_FIXTURE__ = state.calm ? calm : fixture;
    w.__AIJ_NOTICES__ = state.notices ?? {};
    w.__AIJ_USER__ = { name: "Sam", initial: "S", computer: "Mac", computers: "Macs" };
    w.__AIJ_TOAST_PAUSED__ = true;
    if (state.offline) w.__AIJ_OFFLINE__ = { since_ms: t0 - 20_000 };
    if (state.fail) w.__AIJ_NOTICES_FAIL__ = state.fail;
    if (state.account) {
      w.__AIJ_FIGUREHEAD__ = linked;
      w.__AIJ_FLEET_PEERS__ = peers;
      w.__AIJ_CLOUD_BACKUP__ = { enabled: true, code_saved: true, schedule: "daily", last: { at_ms: t0 - 2 * 3_600_000, ok: true, size_bytes: 1_200_000, error: null }, uploading: false, limit_bytes: 25 * 1024 * 1024 };
    }
    localStorage.setItem("aij.theme", state.theme ?? "dark");
    localStorage.setItem("aij.onboarding.dismissed", "1");
    sessionStorage.setItem("aij.ready.skipped", "1");
    Date.now = () => t0;
  }, { t0: T0, fixture: structuredClone(allStatuses), calm: structuredClone(CALM), linked: LINKED, peers: [PEER, { ...PEER, machine_id: "machine-air", label: "MacBook Air" }] });
}

async function show(page: Page, state: State): Promise<void> {
  if (page.url() === "about:blank" || !page.url().startsWith("http")) await page.goto("/");
  await page.evaluate((value) => sessionStorage.setItem("aij.gallery.state", JSON.stringify(value)), state);
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: state.theme });
  await page.reload();
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect.soft(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function shoot(page: Page, name: string, theme: string, clip?: { x: number; y: number; width: number; height: number }): Promise<void> {
  await page.screenshot({ path: resolve(OUT, `${name}-1280x800-${theme}.png`), clip });
  await axe(page, `${name} ${theme}`);
}

/** The menu-bar item's menu, as the builder makes it, drawn like the
 * system's own menu (the system draws the real one). */
function trayPage(theme: "dark" | "light"): string {
  const menu = JSON.parse(readFileSync(TRAY, "utf8")) as { title: string | null; tooltip: string; lines: { type: string; id?: string; text?: string }[] };
  const dark = theme === "dark";
  const lines = menu.lines
    .map((line) => {
      if (line.type === "separator") return `<li role="separator" class="sep"></li>`;
      if (line.type === "heading") return `<li role="none" class="head">${line.text}</li>`;
      if (line.type === "note") return `<li role="none" class="note">${line.text}</li>`;
      return `<li role="menuitem" class="item">${line.text}</li>`;
    })
    .join("");
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>The menu-bar item</title><style>
    body{margin:0;font:13px -apple-system,"Segoe UI",sans-serif;background:${dark ? "#1e1e22" : "#e9e9ee"};color:${dark ? "#ececec" : "#1d1d1f"}}
    .bar{height:26px;display:flex;justify-content:flex-end;align-items:center;gap:14px;padding:0 14px;background:${dark ? "#2a2a2e" : "#f6f6f8"};border-bottom:1px solid ${dark ? "#000" : "#ccc"}}
    .tray{display:flex;align-items:center;gap:4px;padding:1px 7px;border-radius:5px;background:${dark ? "#ffffff26" : "#0000001a"};font-weight:600}
    .dot{width:14px;height:14px;border-radius:4px;background:linear-gradient(135deg,#7c7cf4,#22d3ee)}
    ul{list-style:none;margin:6px 0 0;padding:5px;width:340px;position:absolute;right:60px;top:26px;border-radius:10px;background:${dark ? "#2c2c30" : "#ffffff"};box-shadow:0 12px 40px #0006;border:1px solid ${dark ? "#444" : "#ccc"}}
    li{padding:4px 10px;border-radius:5px}.item:hover{background:#3478f6;color:#fff}
    .head{font-size:12px;font-weight:600;color:${dark ? "#b3b3b8" : "#5a5a60"};padding-top:6px}
    .note{color:${dark ? "#b3b3b8" : "#5a5a60"}}.sep{height:1px;padding:0;margin:5px 8px;background:${dark ? "#4a4a50" : "#d6d6da"}}
  </style></head><body><main><div class="bar" aria-label="The menu bar"><span>Wi-Fi</span><span class="tray" title="${menu.tooltip}"><span class="dot" aria-hidden="true"></span>${menu.title ?? ""}</span><span>Tue 9:41</span></div>
  <ul role="menu" aria-label="AI Juggler">${lines}</ul></main></body></html>`;
}

test("notifications and your menu — every state, both themes", gate("96-notification-centre"), async ({ page }) => {
  test.setTimeout(240_000);
  await page.setViewportSize({ width: 1280, height: 800 });
  await prepare(page);
  for (const theme of ["dark", "light"] as const) {
    // The header: the bell's one count (Home's Needs you count), your menu.
    await show(page, { theme, notices: POPULATED });
    // "3 sessions and 2 more things need you": the count is both. D-49:
    // the list is the backend's (here its stand-in), there once its
    // sessions are.
    await expect(page.getByTestId("home-heading")).toContainText(/sessions? (?:and|needs?)/);
    const heading = (await page.getByTestId("home-heading").textContent()) ?? "";
    const home = [...heading.matchAll(/(\d+) (?:sessions?|more|things?)/g)].reduce((sum, match) => sum + Number(match[1]), 0);
    await expect(page.getByTestId("bell-count")).toHaveText(String(home));
    // The rail's Needs you is the same count.
    await expect(page.getByTestId("rail-needs-you")).toContainText(String(home));
    await shoot(page, "header-badge", theme, { x: 264, y: 0, width: 1016, height: 80 });

    // The centre, populated: Needs action mirrors Needs you; Earlier.
    await page.getByTestId("notification-bell").click();
    await expect(page.getByTestId("notices-needs")).toContainText("Work Max is almost out of room");
    await expect(page.getByTestId("notices-earlier")).toContainText("crashed and was restarted (try 1 of 3)");
    await expect(page.getByText(`Needs action · ${home}`)).toBeVisible();
    await shoot(page, "centre-populated", theme);
    // A need's ⋯: Snooze 1 h, Snooze until tomorrow, Mute this project, Mute this kind.
    await page.getByTestId("notice-more").first().click();
    await expect(page.getByRole("menu").getByRole("menuitem")).toHaveCount(4);
    await shoot(page, "centre-row-menu", theme);
    // A result with Undo (the snooze), from that menu.
    await page.getByTestId("notice-snooze-hour").click();
    await expect(page.getByTestId("toast")).toContainText("Snoozed for an hour");
    await shoot(page, "toast-undo", theme);

    // An error stays until closed, with Retry.
    await show(page, { theme, notices: POPULATED, fail: "AI Juggler couldn't save that just now." });
    await page.getByTestId("notification-bell").click();
    await page.getByTestId("notice-more").first().click();
    await page.getByTestId("notice-snooze-hour").click();
    await expect(page.getByTestId("toast")).toHaveAttribute("data-tone", "error");
    await shoot(page, "toast-error", theme);

    // Empty, and muted (the quiet hours).
    await show(page, { theme, notices: {}, calm: true });
    await page.getByTestId("notification-bell").click();
    await expect(page.getByTestId("notices-empty")).toContainText("You're all caught up.");
    await shoot(page, "centre-empty", theme);
    await show(page, { theme, notices: { ...POPULATED, quiet_until: "09:00" } });
    await page.getByTestId("notification-bell").click();
    await expect(page.getByTestId("notices-muted")).toContainText("Muted until 09:00 — Needs you still tracks");
    await shoot(page, "centre-muted", theme);

    // The banners: an account almost out of room (on a session's page; Home
    // lists it in Needs you), and AI Juggler's own service not answering.
    await show(page, { theme, notices: POPULATED });
    await page.locator("[data-instance-id]").first().click();
    await expect(page.getByTestId("quota-banner")).toContainText("7% of its 5-hour window left");
    await shoot(page, "banner-quota", theme);
    await show(page, { theme, notices: POPULATED, offline: true });
    await expect(page.getByTestId("offline-banner")).toContainText("Showing the last state AI Juggler knew");
    await shoot(page, "banner-offline", theme);

    // Your menu: without a Figurehead account, and with one.
    await show(page, { theme, notices: POPULATED });
    await page.getByTestId("user-menu-button").click();
    await expect(page.getByTestId("user-menu-who")).toContainText("no Figurehead account");
    await shoot(page, "user-menu", theme);
    await show(page, { theme, notices: POPULATED, account: true });
    await page.getByTestId("user-menu-button").click();
    await expect(page.getByTestId("user-menu-who")).toContainText("s…@example.com · Figurehead");
    await expect(page.getByTestId("user-menu-disconnect")).toBeVisible();
    await shoot(page, "user-menu-account", theme);

    // The menu-bar item's menu, from the builder's output.
    await page.setContent(trayPage(theme));
    await shoot(page, "tray-menu", theme);
    await page.goto("/");
  }
});
