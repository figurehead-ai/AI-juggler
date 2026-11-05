/**
 * AJ-E3511, AJ-E3512 (D-78) vitest — Settings › The Floor (each choice a
 * floor.* key, Apps per floor 4–16, Re-create asks first and says what
 * changes and what stays), the Floor button's menu, and the Floor list:
 * grouped by floor in the Floor's order, each state a glyph and a word,
 * a new "waiting on you" said once.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { FloorList, GLYPH, groups, summary } from "./FloorList";
import { FloorPane } from "../../screens/settings/FloorPane";
import { FloorButton } from "../../components/FloorButton";
import type { FloorApp, FloorSnapshot } from "../../lib/ipc/floor";

const w = () => window as unknown as Record<string, unknown>;
afterEach(() => {
  cleanup();
  for (const key of ["__AIJ_FLOOR_SETTINGS__", "__AIJ_FLOOR_PLACES__"]) delete w()[key];
});

const app = (id: string, name: string, sessions: FloorApp["sessions"], extra: Partial<FloorApp> = {}): FloorApp => ({
  id, name, tool: "claude-code", host_app: null, logo: null, kind: "cli", family: "workshop", colour_slot: 0, account_id: "acc", initials: "CC",
  open: sessions.length > 0, last_active_ms: 1_000, cpu_pct: 0, hot_since_ms: null, sessions, ...extra,
} as FloorApp);
const session = (key: string, title: string, state: FloorApp["sessions"][number]["state"], word: string) => ({ key, title, state, word, subagents: 0, project: "shop", tasks: 0, cpu_pct: 0, started_ms: 0, last_ms: 0 });
const SNAPSHOT = {
  at_ms: 10_000,
  apps: [
    app("b", "Codex · Home", [session("s3", "Search index", "working", "Working")]),
    app("a", "Claude Code · Work", [session("s1", "Checkout copy", "asks", "Asks you"), session("s2", "Refund flow", "working", "Working")]),
    app("c", "OpenCode · Lab", [], { open: false }),
  ],
  accounts: [{ id: "acc", label: "Work Max", left_pct: 62, resets_ms: null, last_used_ms: null, out: false }],
  pools: [],
  machine: {},
  notices: 0,
} as unknown as FloorSnapshot;
const PLACES = [
  { app_id: "a", floor: 0, order: 0 },
  { app_id: "b", floor: 0, order: 1 },
  { app_id: "c", floor: 1, order: 2 },
];

describe("AJ-E3512 the Floor list", () => {
  it("groups by floor in the Floor's order, and sums it up", () => {
    const list = groups(SNAPSHOT, PLACES);
    expect(list.map((g) => [g.floor, g.apps.map((a) => a.id)])).toEqual([[0, ["a", "b"]], [1, ["c"]]]);
    expect(summary(list)).toBe("2 floors · 3 Apps · 1 waiting on you");
    expect(GLYPH.asks).toBe("!");
  });

  it("shows each state as a glyph and a word, and says a new wait once", async () => {
    w().__AIJ_FLOOR_PLACES__ = PLACES;
    const view = render(<FloorList />);
    await waitFor(() => expect(view.getByTestId("floor-list-summary").textContent).not.toBe("Reading the office…"));
    act(() => void window.dispatchEvent(new CustomEvent("aij:floor-changed", { detail: SNAPSHOT })));
    await waitFor(() => expect(view.getAllByTestId("floor-list-floor")).toHaveLength(2));
    const asks = view.getAllByTestId("floor-list-row").find((row) => row.dataset.state === "asks")!;
    expect(asks.textContent).toContain("!Asks you");
    expect(view.getAllByTestId("floor-list-app")[0]!.textContent).toContain("Work Max · 62% left");
    expect(view.getAllByTestId("floor-list-app")[2]!.textContent).toContain("No sessions");
    const next = { ...SNAPSHOT, apps: [SNAPSHOT.apps[0]!, { ...SNAPSHOT.apps[1]!, sessions: [...SNAPSHOT.apps[1]!.sessions, session("c:acc/s4", "Flaky test", "asks", "Asks you"), session("s:i9", "Run by AI Juggler", "asks", "Asks you")] }, SNAPSHOT.apps[2]!] };
    act(() => void window.dispatchEvent(new CustomEvent("aij:floor-changed", { detail: next })));
    await waitFor(() => expect(view.getByTestId("floor-list-said").textContent).toBe("Flaky test in Claude Code · Work asks you."));
    // A session AI Juggler runs is said by the main window's AttentionAnnouncer, not twice.
    expect(view.getByTestId("floor-list-said").textContent).not.toContain("Run by AI Juggler");
  });
});

describe("AJ-E3511 Settings › The Floor", () => {
  it("each choice is a floor.* key; Apps per floor stays within 4–16", async () => {
    w().__AIJ_FLOOR_SETTINGS__ = { open_at_start: false, on_top: false, apps_per_floor: 15, motion: "system", decay: true, hidden: ["c"] };
    const view = render(<FloorPane />);
    await waitFor(() => expect(view.getByTestId("floor-per-floor-value").textContent).toBe("15"));
    fireEvent.click(view.getByRole("button", { name: "More Apps per floor" }));
    await waitFor(() => expect(view.getByTestId("floor-per-floor-value").textContent).toBe("16"));
    expect(view.getByRole("button", { name: "More Apps per floor" })).toHaveProperty("disabled", true);
    fireEvent.click(within(view.getByTestId("floor-motion")).getByRole("radio", { name: "Calm" }));
    await waitFor(() => expect((w().__AIJ_FLOOR_SETTINGS__ as { motion: string }).motion).toBe("calm"));
    expect(view.getByTestId("floor-hidden").textContent).toContain("Apps hidden from the Floor (1)");
  });

  it("Re-create asks first, saying what changes and what stays", async () => {
    const view = render(<FloorPane />);
    fireEvent.click(await view.findByTestId("floor-recreate-open"));
    const sheet = view.getByTestId("floor-recreate-sheet");
    expect(sheet.textContent).toContain("Changes");
    expect(sheet.textContent).toContain("Your Apps, sessions, history, accounts and the main window.");
    expect(sheet.textContent).toContain("How the Floor works");
    fireEvent.click(within(sheet).getByRole("button", { name: "Cancel" }));
    expect(view.queryByTestId("floor-recreate-sheet")).toBeNull();
  });
});

describe("AJ-E3511 the Floor button", () => {
  it("is a split button: its arrow opens the Floor, the list and its settings", () => {
    const view = render(<FloorButton />);
    fireEvent.click(view.getByTestId("floor-menu-button"));
    const menu = view.getByTestId("floor-menu");
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Open the Floor⌘⇧M", "The Floor list", "Floor settings…"]);
    const opened: string[] = [];
    const listen = (event: Event) => opened.push((event as CustomEvent<{ id: string }>).detail.id);
    window.addEventListener("aij:open-screen", listen);
    fireEvent.click(view.getByTestId("floor-menu-list"));
    window.removeEventListener("aij:open-screen", listen);
    expect(opened).toEqual(["floor"]);
  });
});
