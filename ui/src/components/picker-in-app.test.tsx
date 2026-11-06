/**
 * AJ-E3105 R1 — a picker of things, rendered where the app uses it, opens
 * on its suggested group: New account › "Start with the same settings
 * as" lists the Apps open now, then the most used, then All, from the
 * relation counts (the backend's, here a stand-in for its answer).
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { EntityKind, RelationCountsInfo } from "../lib/ipc/relations";
import { account } from "../test/fixtures";

// The counts come from the backend only; this file stands in for it.
vi.mock("../lib/bridge", async (original) => ({ ...(await original<typeof import("../lib/bridge")>()), hasLiveBackend: () => true }));
vi.mock("../lib/ipc/accountSettings", async (original) => ({
  ...(await original<typeof import("../lib/ipc/accountSettings")>()),
  accountSettingsCheck: async (_target: string | null, source: string) => ({ from: source, to: "the new account", offers: [], already_there: 0 }),
}));
const asked = vi.hoisted(() => ({ ids: [] as string[] }));
vi.mock("../lib/ipc/relations", async (original) => ({
  ...(await original<typeof import("../lib/ipc/relations")>()),
  relationsCounts: async (entities: { kind: EntityKind; id: string }[]): Promise<RelationCountsInfo[]> => {
    asked.ids = entities.map((entity) => entity.id);
    const sessions = (open: number, all: number) => ({ all, open, ended: all - open, archived: 0 });
    const by: Record<string, [number, number]> = { "f-lab": [2, 9], "f-work": [0, 40], "f-side": [0, 12] };
    return entities.map(({ kind, id }): RelationCountsInfo => ({ kind, id, sessions: sessions(...(by[id] ?? [0, 0])), projects: 0, folders: 0, apps: 0, subscriptions: 0, tools: 0, machines: 0, launch_settings: 0, pools: 0 }));
  },
}));

afterEach(cleanup);

const folder = (id: string, label: string) => account({ id, label });
const SOURCES = [folder("f-a", "Alpha"), folder("f-b", "Beta"), folder("f-work", "Work"), folder("f-c", "Gamma"), folder("f-lab", "Lab"), folder("f-side", "Side"), folder("f-d", "Delta")];

describe("AJ-E3105 R1 — a picker in the app opens on its suggested group", () => {
  it("Start with the same settings as: Open now, then Most used, then All", async () => {
    const { StartWithSettings } = await import("./StartWithSettings");
    render(<StartWithSettings sources={SOURCES} onChange={() => {}} />);
    await waitFor(() => expect(asked.ids.sort()).toEqual(SOURCES.map((source) => source.id).sort()));
    const button = await screen.findByRole("button", { name: /App to copy the settings from/ });
    // The counts are read before the menu opens.
    await new Promise((resolve) => setTimeout(resolve, 20));
    fireEvent.click(button);
    const menu = await screen.findByRole("menu");
    const lines = [...menu.querySelectorAll<HTMLElement>("[role='menuitemradio'], [role='menuitem'], span.font-semibold")]
      .filter((node) => node.closest("[role='menu']") === menu && !node.closest("[role='menuitemradio'] span.font-semibold, [role='menuitem'] span.font-semibold"))
      .map((node) => (node.getAttribute("role") ? node.querySelector("[data-label]")?.textContent ?? node.textContent : `# ${node.textContent}`));
    expect(lines[0]).toBe("# Open now");
    expect(lines[1]).toBe("Lab");
    expect(lines[2]).toBe("# Most used");
    expect(lines.slice(3, 5)).toEqual(["Work", "Side"]);
    expect(lines[5]).toBe("# All");
    // All lists the rest, never one a group already shows.
    expect(lines.slice(6).sort()).toEqual(["Alpha", "Beta", "Delta", "Gamma"]);
  });
});
