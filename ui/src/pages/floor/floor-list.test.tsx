/**
 * AJ-E3511, AJ-E3512 (D-78) vitest — Settings › The Floor (each choice a
 * floor.* key, Apps per floor 4–16, Re-create asks first and says what
 * changes and what stays), the Floor button's menu, and the Floor list:
 * grouped by floor in the Floor's order, each state a glyph and a word,
 * a new "waiting on you" said once.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { FloorList, floorColumns, floorRows, GLYPH, groups, summary, waitingFirst } from "./FloorList";
import { forgetPaged } from "../../lib/paged";
import { FloorPane } from "../../screens/settings/FloorPane";
import { FloorButton } from "../../components/FloorButton";
import type { FloorApp, FloorSnapshot } from "../../lib/ipc/floor";

const w = () => window as unknown as Record<string, unknown>;
afterEach(() => {
  cleanup();
  forgetPaged();
  for (const key of ["__AIJ_FLOOR_SETTINGS__", "__AIJ_FLOOR_PLACES__", "__AIJ_FLOOR_SAID__"]) delete w()[key];
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
  needs: 0,
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
    expect(summary(list, 0)).toBe("2 floors · 3 Apps · 1 session waiting");
    // AJ-E2502: Needs you's one count leads, as Home and the bell say it
    expect(summary(list, 4)).toBe("2 floors · 3 Apps · 4 need you · 1 session waiting");
    expect(summary(list, 1)).toBe("2 floors · 3 Apps · 1 needs you · 1 session waiting");
    expect(GLYPH.asks).toBe("!");
    expect(waitingFirst(SNAPSHOT.apps[1]!.sessions.slice().reverse()).map((s) => s.key)).toEqual(["s1", "s2"]);
  });

  // FL1 (rc31): the list opened on an App of ended rows; what waits on
  // you leads, on each floor and in each App, and ended rows go last.
  it("leads with what waits on you: its Apps first, then live rows, ended last", () => {
    const snapshot = {
      ...SNAPSHOT,
      apps: [
        app("old", "Claude Code · Old", [session("e1", "Old run", "ended", "Ended"), session("e2", "Older run", "ended", "Ended")]),
        app("busy", "Claude Code · Busy", [session("e3", "Gone", "ended", "Ended"), session("w1", "Build", "working", "Working"), session("l1", "Out", "limit", "Limit reached")]),
      ],
    };
    const places = [
      { app_id: "old", floor: 0, order: 0 },
      { app_id: "busy", floor: 0, order: 1 },
    ];
    const [floor] = groups(snapshot, places);
    expect(floor!.apps.map((a) => a.id)).toEqual(["busy", "old"]);
    expect(floorRows(floor!, "", false).map((row) => (row.kind === "app" ? row.app.id : row.session.key))).toEqual(["busy", "l1", "w1", "e3", "old", "e1", "e2"]);
  });

  it("shows each state as a glyph and a word, and says a new wait once", async () => {
    w().__AIJ_FLOOR_PLACES__ = PLACES;
    const view = render(<FloorList />);
    await waitFor(() => expect(view.getByTestId("floor-list-summary").textContent).not.toBe("Reading the office…"));
    act(() => void window.dispatchEvent(new CustomEvent("aij:floor-changed", { detail: SNAPSHOT })));
    await waitFor(() => expect(view.getAllByTestId("floor-list-floor")).toHaveLength(2));
    const asks = view.getAllByTestId("floor-list-row").find((row) => row.dataset.state === "asks")!;
    expect(asks.textContent).toContain("!Asks you");
    expect(view.getAllByTestId("floor-list-accounts")[0]!.textContent).toBe("Work Max · 62% left");
    expect(view.getAllByTestId("floor-list-app")[2]!.textContent).toContain("Unused");
    const next = { ...SNAPSHOT, apps: [SNAPSHOT.apps[0]!, { ...SNAPSHOT.apps[1]!, sessions: [...SNAPSHOT.apps[1]!.sessions, session("c:acc/s4", "Flaky test", "asks", "Asks you"), session("s:i9", "Run by AI Juggler", "asks", "Asks you")] }, SNAPSHOT.apps[2]!] };
    act(() => void window.dispatchEvent(new CustomEvent("aij:floor-changed", { detail: next })));
    await waitFor(() => expect(view.getByTestId("floor-list-said").textContent).toBe("Flaky test in Claude Code · Work asks you."));
    // A session AI Juggler runs is said by the main window's AttentionAnnouncer, not twice.
    expect(view.getByTestId("floor-list-said").textContent).not.toContain("Run by AI Juggler");
  });
});

async function shown(snapshot: FloorSnapshot) {
  w().__AIJ_FLOOR_PLACES__ = PLACES;
  const view = render(<FloorList />);
  await waitFor(() => expect(view.getByTestId("floor-list-summary").textContent).not.toBe("Reading the office…"));
  act(() => void window.dispatchEvent(new CustomEvent("aij:floor-changed", { detail: snapshot })));
  await waitFor(() => expect(view.getAllByTestId("floor-list-floor")).toHaveLength(2));
  return view;
}
const titles = (view: ReturnType<typeof render>) => view.queryAllByTestId("floor-list-row").map((row) => row.textContent);

describe("the 0.4 review of the Floor list", () => {
  it("one header per floor, the room once, no empty columns, and the App's own mark (43, 81, 67)", async () => {
    const view = await shown(SNAPSHOT);
    // One column header per floor (floor 1 has two Apps), and only the columns with a value on it.
    const heads = view.getAllByTestId("floor-list-columns");
    expect(heads).toHaveLength(2);
    expect(heads[0]!.textContent).toBe("StateSessionProject");
    expect(floorColumns(groups(SNAPSHOT, PLACES)[0]!)).toEqual({ project: true, subagents: false, tasks: false });
    expect(view.getByTestId("floor-list").textContent).not.toContain("—");
    // Both Apps on floor 1 share one Account: its room is said once, on the floor.
    const floor = view.getAllByTestId("floor-list-floor")[0]!;
    expect(floor.textContent!.split("62% left").length - 1).toBe(1);
    for (const app of within(floor).getAllByTestId("floor-list-app")) expect(app.textContent).not.toContain("left");
    // The App's icon and its profile's dot, not initials or a style name.
    const mark = within(floor).getAllByTestId("floor-list-app-mark")[0]!;
    expect(within(mark).getByTestId("app-icon")).toBeTruthy();
    expect(within(mark).getByTestId("profile-badge").getAttribute("title")).toBe("Work");
    expect(floor.textContent).not.toContain("CC");
    expect(floor.textContent).not.toMatch(/workshop/i);
    // Show on the Floor once per App, and each row is the action too.
    expect(within(floor).getAllByRole("button", { name: /^Show on the Floor: / })).toHaveLength(2);
    expect(view.getAllByTestId("floor-list-row")[0]!.getAttribute("title")).toBe("Show on the Floor (↩)");
  });

  it("searches, and the Waiting on you chip leaves what waits (22)", async () => {
    const view = await shown(SNAPSHOT);
    const chip = view.getByTestId("floor-list-waiting");
    expect(chip.textContent).toBe("Waiting on you 1");
    fireEvent.change(view.getByTestId("floor-list-search"), { target: { value: "refund" } });
    await waitFor(() => expect(titles(view)).toEqual(["▸Working, Refund flow, project shop. Show on the Floor"]));
    // An App whose name matches keeps all its sessions.
    fireEvent.change(view.getByTestId("floor-list-search"), { target: { value: "codex" } });
    await waitFor(() => expect(view.getAllByTestId("floor-list-row").map((row) => row.dataset.state)).toEqual(["working"]));
    fireEvent.change(view.getByTestId("floor-list-search"), { target: { value: "" } });
    fireEvent.click(chip);
    expect(chip.getAttribute("aria-pressed")).toBe("true");
    await waitFor(() => expect(view.getAllByTestId("floor-list-row").map((row) => row.dataset.state)).toEqual(["asks"]));
    // Floor 2 holds nothing waiting: it goes.
    expect(view.getAllByTestId("floor-list-floor")).toHaveLength(1);
    fireEvent.change(view.getByTestId("floor-list-search"), { target: { value: "nowhere" } });
    await waitFor(() => expect(view.getByTestId("floor-list-none").textContent).toContain("Nothing on the Floor matches"));
    expect(floorRows(groups(SNAPSHOT, PLACES)[1]!, "", true)).toEqual([]);
  });

  it("a floor is the shared paged list: a page at a time, not every session at once", async () => {
    const many = Array.from({ length: 140 }, (_, i) => session(`m${i}`, `Task ${i}`, "working", "Working"));
    const big = { ...SNAPSHOT, apps: [{ ...SNAPSHOT.apps[0]!, sessions: many }, SNAPSHOT.apps[1]!, SNAPSHOT.apps[2]!] } as FloorSnapshot;
    const view = await shown(big);
    await waitFor(() => expect(view.getByTestId("floor-list-0-more")).toBeTruthy());
    expect(view.getAllByTestId("floor-list-row").length).toBeLessThan(60);
  });
});

describe("AJ-E3517 the list carries the card's words", () => {
  it("resting on a row reads what it asks (or did last) after a moment, and it goes a moment after leaving", async () => {
    w().__AIJ_FLOOR_PLACES__ = PLACES;
    w().__AIJ_FLOOR_SAID__ = { s1: { asks: "Should I also update the snapshot test?", step: null }, s2: { asks: null, step: "ran \"pnpm test\"" } };
    const view = render(<FloorList />);
    await waitFor(() => expect(view.getByTestId("floor-list-summary").textContent).not.toBe("Reading the office…"));
    act(() => void window.dispatchEvent(new CustomEvent("aij:floor-changed", { detail: SNAPSHOT })));
    const rowOf = (title: string) => view.getAllByTestId("floor-list-row").find((r) => r.textContent?.includes(title))!;
    await waitFor(() => expect(rowOf("Checkout copy")).toBeTruthy());
    const asks = rowOf("Checkout copy");
    const bar = () => view.getByTestId("floor-list-row-said");
    expect(bar().getAttribute("aria-live")).toBe("polite");
    act(() => asks.focus());
    expect(document.activeElement).toBe(asks);
    expect(bar().textContent).toBe("");
    await waitFor(() => expect(bar().textContent).toBe("Checkout copy · Asks: Should I also update the snapshot test?"));
    const working = view.getAllByTestId("floor-list-row").find((row) => row.textContent?.includes("Refund flow"))!;
    act(() => asks.blur());
    fireEvent.pointerOver(working);
    await waitFor(() => expect(bar().textContent).toBe('Refund flow · Last: ran "pnpm test"'));
    fireEvent.pointerOut(working);
    await waitFor(() => expect(bar().textContent).toBe(""));
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

  it("D-80: the Floor's look, Pixel art by default, Full resolution kept as chosen", async () => {
    w().__AIJ_FLOOR_SETTINGS__ = { open_at_start: false, on_top: false, apps_per_floor: 10, motion: "system", detail: "pixel", decay: true, hidden: [] };
    const view = render(<FloorPane />);
    const look = await view.findByTestId("floor-detail");
    await waitFor(() => expect(within(look).getByRole("radio", { name: "Pixel art" }).getAttribute("aria-checked")).toBe("true"));
    expect(look.textContent).toContain("Full resolution:");
    fireEvent.click(within(look).getByRole("radio", { name: "Full resolution" }));
    await waitFor(() => expect((w().__AIJ_FLOOR_SETTINGS__ as { detail: string }).detail).toBe("full"));
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
