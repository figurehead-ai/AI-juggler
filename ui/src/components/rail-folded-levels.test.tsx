/**
 * D-117 (AJ-E3603), CI run 43768776781's 69-rail-narrow capture: the folded
 * icon rail's count read a red ✕5 while no icon was lit red. Its two
 * broken sessions (L3, stopped with an error) had ended, and the column
 * showed only what is open, so they had no icon at all; only the three L2
 * ones were lit. Now what waits on you or broke (L2/L3) has its icon, open
 * or not: lit, its corner amber "!" at L2, red "✕" at L3 (an App with both
 * shows the higher), the glyph hidden and the name leading with the word.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { needsFor } from "../test/needs-kit";
import { FoldedRail, foldedOrder, foldedShows } from "./RailFolded";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationsProvider } from "../lib/conversations";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { resetSeen } from "../lib/seen";
import { resetNoticeStandIn } from "../lib/ipc/notices";
import { resetNotices } from "../lib/notices-store";
import { itemsOf, openNow } from "../lib/rail-items";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

const NOW = new Date(2026, 10, 15, 12, 0, 0).getTime();
const projects: ProjectInfo[] = [{ id: "proj", root: "/Users/you/Dev/shop", name: "shop", last_activity_ms: NOW, active_instance: null, accounts_used: [], instance_count: 5, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false }];

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "claude-a", account_label: "Alpha", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/you/Dev/shop", kind: "terminal", isolation_label: null, project_id: "proj", conversation_id: null, ...over,
  };
}

const failed = { status: "failed", needs_attention: "failed" } as const;
/** Four Apps (an App is its tool and Account): L2 only, L3 only (a desktop
 * one, as the fixture's "F" App), both, and calm. */
const APPS = {
  l2: [own("asks", { status: "waiting_user", needs_attention: "waiting_user" })],
  l3: [own("desk-broke", { ...failed, provider: "fake-desktop", kind: "desktop", account_id: "fake-desk", account_label: "Fake Desk" })],
  both: [own("both-stuck", { status: "stuck", needs_attention: "stuck", account_id: "claude-b", account_label: "Beta" }), own("both-broke", { ...failed, account_id: "claude-b", account_label: "Beta" })],
  calm: [own("calm", { provider: "codex", account_id: "codex-c", account_label: "Gamma" })],
};
const SESSIONS = Object.values(APPS).flat();

beforeEach(() => {
  resetSeen();
  resetRailOrderCache();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
  resetNoticeStandIn();
  resetNotices();
});

async function folded() {
  Object.assign(window, { __AIJ_FIXTURE__: { conversations: [] } });
  needsFor({ sessions: SESSIONS, conversations: [], projects });
  const view = render(
    <ActionHostProvider host={{}}>
      <ConversationsProvider>
        <FoldedRail sessions={SESSIONS} projects={projects} now={NOW} selectedId={null} onSelect={() => {}} />
      </ConversationsProvider>
    </ActionHostProvider>,
  );
  const rail = await view.findByTestId("rail-folded");
  await waitFor(() => expect(rail.querySelector("[data-instance-id='desk-broke']")).toBeTruthy());
  return { view, rail };
}

/** An App's icons: whether each is lit, its corner, and its spoken name. */
function icons(rail: HTMLElement, sessions: InstanceInfo[]) {
  return sessions.map((session) => {
    const row = rail.querySelector<HTMLElement>(`button[data-instance-id='${session.id}']`);
    expect(row, session.id).toBeTruthy();
    const corner = row!.querySelector<HTMLElement>("[data-testid='rail-folded-needs']");
    if (corner) expect(corner.getAttribute("aria-hidden")).toBe("true");
    return {
      lit: row!.querySelector(".app-tone")!.classList.contains("is-lit"),
      // AJ-E3603: its corner is the level's small mark, never a filled
      // badge: a red ✕ (L3, drawn by `.level-mark`) or an amber dot (L2).
      corner: corner === null ? null : corner.querySelector(".level-mark")?.getAttribute("data-level") === "3" ? "✕" : "•",
      level: corner?.getAttribute("data-level") ?? null,
      name: row!.querySelector(".sr-only")!.textContent ?? "",
    };
  });
}

/** The App's corner: its highest level's (✕ over •). */
const appCorner = (marks: { corner: string | null }[]) => (marks.some((mark) => mark.corner === "✕") ? "✕" : marks.some((mark) => mark.corner === "•") ? "•" : null);

describe("the folded icon rail lights L2 and L3 alike (D-117)", () => {
  it("a broken session has ended, yet keeps its icon there; a calm ended one doesn't", () => {
    const items = itemsOf({ sessions: [...SESSIONS, own("done", { status: "finished" })], conversations: [], projects, seen: {} });
    const broke = items.find((item) => item.key === "s:desk-broke")!;
    expect(openNow(broke)).toBe(false);
    expect(foldedShows(broke)).toBe(true);
    expect(foldedShows(items.find((item) => item.key === "s:done")!)).toBe(false);
    expect(foldedOrder(items, "yours", [], []).map((item) => item.key).sort()).toEqual(SESSIONS.map((session) => `s:${session.id}`).sort());
  });

  it("each App: lit at L2 or L3 with its level's corner, the higher when both; calm stays muted; names lead with the word", async () => {
    const { rail } = await folded();
    const l2 = icons(rail, APPS.l2);
    expect(l2).toEqual([{ lit: true, corner: "•", level: "2", name: expect.stringMatching(/^Asks you, /) }]);
    const l3 = icons(rail, APPS.l3);
    expect(l3).toEqual([{ lit: true, corner: "✕", level: "3", name: expect.stringMatching(/^Stopped with an error, /) }]);
    const both = icons(rail, APPS.both);
    expect(both).toEqual([
      { lit: true, corner: "•", level: "2", name: expect.stringMatching(/^Probably stuck, /) },
      { lit: true, corner: "✕", level: "3", name: expect.stringMatching(/^Stopped with an error, /) },
    ]);
    expect(appCorner(both)).toBe("✕");
    expect(appCorner(l2)).toBe("•");
    expect(appCorner(l3)).toBe("✕");
    const calm = icons(rail, APPS.calm);
    expect(calm).toEqual([{ lit: false, corner: null, level: null, name: expect.stringMatching(/^Working, /) }]);
    // Every lit icon is one the count holds: as many red corners as broke.
    expect(rail.querySelectorAll("[data-testid='rail-folded-needs'][data-level='3']")).toHaveLength(2);
    expect(rail.querySelectorAll("[data-testid='rail-folded-needs'][data-level='2']")).toHaveLength(2);
  });

  it("the count: L2+L3, its level's glyph hidden, what broke said in its name", async () => {
    const { view } = await folded();
    const count = await view.findByTestId("rail-folded-needs-you");
    await waitFor(() => expect(count.getAttribute("aria-label")).toBe("Needs you: 4 things need you, 2 stopped with an error. Open Home"));
    expect(count.getAttribute("data-level")).toBe("3");
    expect(count.querySelector(".level-glyph")!.getAttribute("aria-hidden")).toBe("true");
    expect(count.textContent).toBe("4");
  });
});
