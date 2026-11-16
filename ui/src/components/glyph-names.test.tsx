/**
 * D-117 (AJ-E3602/E3603), the a11y fix of 2026-11-15: Happo's snapshots of
 * 83f700ed6 heard the rail's head as "Needs you ✕ 5" (VoiceOver: "multiplication
 * x"). A level's glyphs (⚠ ✕, the dot, the ring, a review's ✓) are decoration,
 * hidden from screen readers; the words say the level ("Needs you 3, 1 stopped
 * with an error, 2 wait on you"). Every accessible name on the rail, Home's
 * tiles and the bell, on fixtures with L1, L2 and L3 things, holds no glyph.
 * (The rendered page's own sweep, generated content included, is in
 * tests/visual/a11y.spec.ts.)
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { needsFor } from "../test/needs-kit";
import { InstanceBoard } from "./WorkflowRail";
import { HomeApps } from "./HomeApps";
import { NotificationBell } from "./NotificationBell";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { resetSeen } from "../lib/seen";
import { itemsOf, type Need } from "../lib/rail-items";
import { applySummary, resetNotices } from "../lib/notices-store";
import { resetNoticeStandIn, standInNeeds, type StandInNeed } from "../lib/ipc/notices";
import { brokenParts, levelCountWords } from "../lib/attention";
import { placesConversations } from "../preview/places";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

/** The status glyphs no accessible name may hold. */
const GLYPHS = /[✕⚠○●◐✓]/u;
/** Every role that can carry a name. */
const ROLES = ["button", "heading", "link", "region", "list", "listitem", "menuitem", "tab", "checkbox", "switch", "meter", "progressbar", "img", "status", "alert", "group", "navigation", "complementary", "dialog", "option", "row", "cell", "article", "term", "definition", "note", "tooltip"] as const;

/** Each named element's role and accessible name under `root` (the
 * testing library's name computation, as a screen reader would read it). */
function names(root: HTMLElement): string[] {
  const out: string[] = [];
  for (const role of ROLES)
    within(root).queryAllByRole(role, {
      name: (name) => {
        if (name) out.push(`${role}: ${name}`);
        return false;
      },
    });
  return out;
}
const withGlyphs = (root: HTMLElement) => names(root).filter((name) => GLYPHS.test(name));

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
/** The preview's fixture seam (`__AIJ_FIXTURE__`). */
const seed = (fixture: unknown) => Object.assign(window, { __AIJ_FIXTURE__: fixture });

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
  setRailView(DEFAULT_VIEW);
  resetRailViewCache();
  resetRailOrderCache();
  resetNoticeStandIn();
  resetNotices();
});

const projects: ProjectInfo[] = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: NOW, active_instance: null, accounts_used: ["Work Max"], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false }];

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  };
}

/** L3 (stopped with an error), L2 (a guess: probably stuck; the preview's
 * two that ask) and L1 (the preview's "Done, review it"), with a calm one. */
const SESSIONS = [own("broke", { status: "failed", needs_attention: "failed" }), own("stuck", { status: "stuck", needs_attention: "stuck" }), own("calm")];

function board(sessions: InstanceInfo[]) {
  const conversations = placesConversations(NOW);
  seed({ conversations });
  needsFor({ sessions, conversations, projects }); // D-49: the backend's list
  return render(
    <ConversationsProvider>
      <InstanceBoard sessions={sessions} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={projects} snooze={{}} onSnooze={() => {}} onSelectProject={() => {}} />
    </ConversationsProvider>,
  );
}

describe("the spoken words of a level count", () => {
  it("say the number, what broke and what waits, never a glyph", () => {
    expect(levelCountWords(3, [])).toBe("3");
    expect(levelCountWords(5, ["stopped with an error", "stopped with an error"])).toBe("5, 2 stopped with an error, 3 wait on you");
    expect(levelCountWords(2, ["stopped with an error"])).toBe("2, 1 stopped with an error, 1 waits on you");
    expect(levelCountWords(1, ["stopped with an error"])).toBe("1, 1 stopped with an error");
    expect(brokenParts(["out of room", "stopped with an error", "stopped with an error"])).toBe("2 stopped with an error, 1 out of room");
  });
});

describe("no status glyph in any accessible name (D-117)", () => {
  it("the rail: Needs you's head says its levels in words, its rows and App headers hold no glyph", async () => {
    const view = board(SESSIONS);
    const needs = await view.findByTestId("rail-needs-you");
    // The glyph is drawn, hidden from screen readers.
    const glyph = needs.querySelector(".level-glyph")!;
    expect(glyph.getAttribute("data-level")).toBe("3");
    expect(glyph.closest("[aria-hidden='true']")).toBe(glyph);
    const shown = Number(/Needs you\s*(\d+)/.exec(needs.textContent ?? "")?.[1]);
    expect(shown).toBeGreaterThan(1);
    const head = within(needs).getByRole("button", { expanded: true });
    expect(head).toBe(within(needs).getByRole("button", { name: `Needs you ${levelCountWords(shown, ["stopped with an error"])}` }));
    // (A browser names the heading from it too: tests/visual/a11y-glyph-names.spec.ts.)
    expect(levelCountWords(shown, ["stopped with an error"])).toMatch(/^\d+, 1 stopped with an error, \d+ wait on you$/);
    // The L3 row: its word in its name, its ✕ drawn but hidden.
    const broke = view.container.querySelector<HTMLElement>("[data-level='3'][data-need='failed']")!;
    expect(broke.textContent).toContain("✕");
    expect(view.getByRole("button", { name: /^Stopped with an error, / })).toBeTruthy();
    // Its App's header says what broke in words (its red dot is decoration).
    expect(view.getAllByRole("button", { name: /need you, 1 stopped with an error/ }).length).toBeGreaterThan(0);
    expect(withGlyphs(view.container)).toEqual([]);
  });

  it("a folded group's count says what broke in words, its red dot hidden", async () => {
    setRailView({ ...DEFAULT_VIEW, group: "project" });
    const view = board(SESSIONS);
    fireEvent.click(await view.findByRole("button", { name: "Fold ai-juggler" }));
    // D-117 (AJ-E3603): the App heads' mark (`HeadNeeds`), folded or not.
    const mark = view.getByTestId("place-needs");
    expect(mark.getAttribute("data-level")).toBe("3");
    expect(mark.querySelector(".level-mark")!.getAttribute("aria-hidden")).toBe("true");
    // In the project's own button: a mark, never a button in a button.
    expect(mark.tagName).toBe("SPAN");
    expect(view.getByTestId("rail-project")).toBe(view.getByRole("button", { name: /^ai-juggler, \d+ sessions, 2 need you, 1 stopped with an error$/ }));
    expect(withGlyphs(view.container)).toEqual([]);
  });

  it("Home's tiles: a tile's count says what broke in words", async () => {
    const conversations = placesConversations(NOW);
    seed({ conversations });
    const base = { sessions: SESSIONS, conversations, projects, seen: {}, snoozed: () => false, pins: new Set<string>() };
    // The backend's needs (D-49): the preview's, and one session in the
    // Claude app's Work profile that stopped with an error (L3).
    const before = itemsOf(base);
    const work = (item: (typeof before)[number]) => item.place.key === "app|claude-app|Work" && item.live;
    const failed = before.find((item) => work(item) && item.need === null)!;
    const needOf = new Map<string, Need>([...before.flatMap((item) => (item.need === null ? [] : [[item.key, item.need] as const])), [failed.key, "failed"]]);
    const view = render(<HomeApps items={itemsOf({ ...base, needOf })} />);
    const tiles = await view.findAllByTestId("home-app-tile");
    const broke = tiles.find((tile) => tile.dataset.place === "app|claude-app|Work")!;
    expect(broke.dataset.level).toBe("3");
    expect(within(broke).getByTestId("home-app-needs").textContent).toBe("2 need you, 1 stopped with an error");
    expect(within(broke).getByTestId("home-app-needs").querySelector(".level-mark")!.getAttribute("aria-hidden")).toBe("true");
    // AI Juggler's own sessions are their CLI App's (with the Terminal's,
    // its folder's): its row says what broke there too.
    const cli = tiles.find((tile) => tile.dataset.place === "cli|claude-code|claude-work")!;
    expect(within(cli).getByTestId("home-app-needs").textContent).toMatch(/need you, 1 stopped with an error$/);
    expect(withGlyphs(view.container)).toEqual([]);
  });

  it("the bell: its red badge is decoration; its name says what broke", async () => {
    const target = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
    const need = (key: string, kind: string, title: string): StandInNeed => ({ key, kind, title, detail: "ai-juggler", project_id: null, project: null, target: { ...target, session_id: key }, action: "open" });
    applySummary(standInNeeds([need("s:1", "asks", "“Docs bot” needs your reply"), need("s:2", "failed", "“Nightly” stopped with an error"), need("s:3", "review", "“Fix” is done")]));
    render(<NotificationBell sessions={[]} />);
    const badge = await screen.findByTestId("bell-count");
    expect(badge.getAttribute("data-level")).toBe("3");
    expect(badge.getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByRole("button", { name: /^Notifications: 2 things need you, 1 stopped with an error/ })).toBeTruthy();
    expect(withGlyphs(document.body)).toEqual([]);
  });
});
