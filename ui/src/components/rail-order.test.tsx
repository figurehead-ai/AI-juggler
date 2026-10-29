/**
 * D-47 — Open now's order as the user meets it: the order menu on its
 * header (three orders with icons and what each does, then Reset my order,
 * with Undo), the same choice in Settings, the keyboard twin of a drag
 * (⌥⌘↑ / ⌥⌘↓, said politely, focus kept), the handle and pin shown on
 * hover and focus only, pinning a session another app runs, a session
 * that needs you staying in its place, and a remembered fold.
 */
import { needsFor } from "../test/needs-kit";
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { RailOrderChoice } from "./RailOrderChoice";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { ORDER_KEY, railOrder, resetRailOrderCache, setOrderMode } from "../lib/rail-order-store";
import { undoLatest } from "./Toast";
import { placesConversations } from "../preview/places";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

const NOW = new Date(2026, 9, 27, 15, 0, 0).getTime();
type Seams = { __AIJ_FIXTURE__?: unknown; __AIJ_APP_SETTINGS__?: Record<string, string> };
const seams = () => window as unknown as Seams;

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_APP_SETTINGS__;
  resetRailViewCache();
  resetRailOrderCache();
  vi.restoreAllMocks();
});

const projects = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler" }] as ProjectInfo[];

function board(sessions: InstanceInfo[] = []) {
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  needsFor({ sessions, conversations: placesConversations(NOW), projects }); // D-49: the backend's list
  return render(
    <ConversationsProvider>
      <InstanceBoard sessions={sessions} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={projects} snooze={{}} onSnooze={() => {}} />
    </ConversationsProvider>,
  );
}

const rowsIn = (root: HTMLElement, group: string) =>
  [...root.querySelectorAll<HTMLElement>(`[data-order-group="${group}"][data-order-row]`)].map((row) => row.dataset.orderRow);

describe("the order menu on Open now's header", () => {
  it("offers Your order, Latest activity and Name with icons and what each does, then Reset my order", async () => {
    const view = board();
    const button = await view.findByTestId("rail-order-button");
    expect(button.getAttribute("aria-label")).toBe("Open now's order: Your order");
    fireEvent.click(button);
    const menu = await view.findByRole("menu", { name: "Order of Open now" });
    const options = within(menu).getAllByRole("menuitemradio");
    expect(options.map((option) => option.textContent)).toEqual([
      expect.stringContaining("Stays put. New ones join at the end. Drag or pin to arrange."),
      expect.stringContaining("Newest event first; holds still under the pointer."),
      expect.stringContaining("A to Z."),
    ]);
    expect(options.map((option) => option.getAttribute("aria-checked"))).toEqual(["true", "false", "false"]);
    // AJ-E2106: an icon on every line; two groups.
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Reset my order"]);
    for (const item of [...options, within(menu).getByRole("menuitem")]) expect(item.querySelector("svg")).not.toBeNull();
    expect(within(menu).getAllByRole("separator")).toHaveLength(1);
    fireEvent.click(within(menu).getByTestId("order-latest"));
    expect(railOrder().mode).toBe("latest");
    await waitFor(() => expect(view.getByTestId("rail-order-button").textContent).toContain("Latest activity"));
    // No handles outside Your order.
    expect(view.queryAllByTestId("order-grip")).toHaveLength(0);
  });

  it("Reset my order forgets the arrangement, with Undo", async () => {
    const view = board();
    await view.findByTestId("rail-order-button");
    await waitFor(() => expect(railOrder().rows.length).toBeGreaterThan(0));
    const before = railOrder().rows;
    fireEvent.click(view.getByTestId("rail-order-button"));
    fireEvent.click(await view.findByTestId("order-reset"));
    expect(railOrder().rows).toEqual([]);
    // Its toast's Undo (⌘Z) puts it back.
    await act(async () => void undoLatest());
    await waitFor(() => expect(railOrder().rows).toEqual(before));
  });

  it("Settings offers the same choice", () => {
    const view = render(<RailOrderChoice />);
    fireEvent.click(view.getByTestId("settings-order-name"));
    expect(railOrder().mode).toBe("name");
    expect(view.getByRole("radio", { name: /Name/ })).toHaveProperty("checked", true);
    expect(view.getByRole("button", { name: "Reset my order" })).toBeTruthy();
  });
});

describe("arranging from the keyboard (the drag's twin)", () => {
  it("⌥⌘↓ and ⌥⌘↑ move the focused row within its place, said politely, focus kept", async () => {
    const view = board();
    const list = await view.findByTestId("place-board");
    await waitFor(() => expect(rowsIn(list, "Claude|Work")).toEqual(["c:claude-work/outside-1", "c:claude-work/outside-3"]));
    const first = list.querySelector<HTMLElement>('[data-order-row="c:claude-work/outside-1"] [data-rail-item]')!;
    expect(first.getAttribute("aria-keyshortcuts")).toBe("Alt+Meta+ArrowUp Alt+Meta+ArrowDown");
    first.focus();
    fireEvent.keyDown(first, { key: "ArrowDown", metaKey: true, altKey: true });
    await waitFor(() => expect(rowsIn(list, "Claude|Work")).toEqual(["c:claude-work/outside-3", "c:claude-work/outside-1"]));
    const said = view.getByTestId("rail-order-said");
    expect(said.getAttribute("aria-live")).toBe("polite");
    expect(said.textContent).toBe("Moved to position 2 of 2");
    await waitFor(() => expect(document.activeElement?.getAttribute("data-rail-item")).toBe("c:claude-work/outside-1"));
    // Ctrl+Alt does the same off a Mac.
    fireEvent.keyDown(document.activeElement!, { key: "ArrowUp", ctrlKey: true, altKey: true });
    await waitFor(() => expect(rowsIn(list, "Claude|Work")).toEqual(["c:claude-work/outside-1", "c:claude-work/outside-3"]));
    // Kept for the next start.
    await waitFor(() => expect(seams().__AIJ_APP_SETTINGS__?.[ORDER_KEY]).toContain("c:claude-work/outside-1"));
  });

  it("a place moves too, and ⌘1–9 follow it", async () => {
    const view = board();
    const list = await view.findByTestId("place-board");
    const heads = () => [...list.querySelectorAll<HTMLElement>("[data-order-head]")].map((head) => head.dataset.orderHead);
    await waitFor(() => expect(heads()).toEqual(["ChatGPT|Codex", "Claude|Personal", "Claude|Work", "Terminal|Work Max"]));
    const terminal = list.querySelector<HTMLElement>('[data-order-head="Terminal|Work Max"] [data-testid="place-page"]')!;
    terminal.focus();
    fireEvent.keyDown(terminal, { key: "ArrowUp", metaKey: true, altKey: true });
    await waitFor(() => expect(heads()).toEqual(["ChatGPT|Codex", "Claude|Personal", "Terminal|Work Max", "Claude|Work"]));
    expect(view.getByTestId("rail-order-said").textContent).toBe("Moved to position 3 of 4");
    expect(list.querySelector('[data-order-head="Terminal|Work Max"] [data-testid="place-page"]')!.getAttribute("aria-keyshortcuts")).toContain("Meta+3");
  });

  it("outside Your order, the keys say how to arrange instead of moving", async () => {
    setOrderMode("name");
    const view = board();
    const list = await view.findByTestId("place-board");
    const row = await waitFor(() => list.querySelector<HTMLElement>("[data-order-row] [data-rail-item]")!);
    fireEvent.keyDown(row, { key: "ArrowDown", metaKey: true, altKey: true });
    expect(view.getByTestId("rail-order-said").textContent).toMatch(/Pick Your order/);
  });
});

describe("rows in Your order", () => {
  it("the handle and pin show on hover and focus only; the handle is the pointer's alone", async () => {
    const view = board();
    const list = await view.findByTestId("place-board");
    const row = await waitFor(() => list.querySelector<HTMLElement>('[data-order-row="c:claude-personal/outside-4"]')!);
    const tools = within(row).getByTestId("row-tools");
    expect(tools.className).toContain("opacity-0");
    expect(tools.className).toContain("group-hover:opacity-100");
    expect(tools.className).toContain("group-focus-within:opacity-100");
    expect(within(row).getByTestId("order-grip").getAttribute("aria-hidden")).toBe("true");
  });

  it("pins a session another app runs: it leads its place, and stays pinned", async () => {
    const view = board();
    const list = await view.findByTestId("place-board");
    await waitFor(() => expect(rowsIn(list, "Claude|Personal")).toEqual(["c:claude-personal/outside-4", "c:claude-personal/outside-5"]));
    const pin = list.querySelector<HTMLElement>('[data-order-row="c:claude-personal/outside-5"] [data-testid="row-pin"]')!;
    expect(pin.getAttribute("aria-pressed")).toBe("false");
    act(() => pin.click());
    await waitFor(() => expect(rowsIn(list, "Claude|Personal")).toEqual(["c:claude-personal/outside-5", "c:claude-personal/outside-4"]));
    expect(railOrder().pins).toEqual(["c:claude-personal/outside-5"]);
    const pinned = list.querySelector<HTMLElement>('[data-order-row="c:claude-personal/outside-5"] [data-testid="row-pin"]')!;
    expect(pinned.getAttribute("aria-pressed")).toBe("true");
    // The row's menu says Unpin.
    fireEvent.contextMenu(list.querySelector<HTMLElement>('[data-order-row="c:claude-personal/outside-5"]')!, { clientX: 5, clientY: 5, button: 2 });
    expect(await view.findByTestId("action-pin")).toHaveProperty("textContent", expect.stringContaining("Unpin"));
  });

  it("one that needs you stays in its place, its state in amber, and is in Needs you too", async () => {
    const view = board();
    const list = await view.findByTestId("place-board");
    const inPlace = await waitFor(() => list.querySelector<HTMLElement>('[data-order-row="c:claude-work/outside-3"]')!);
    const word = within(inPlace).getByTestId("state-word");
    expect(word.textContent).toBe("Asks you");
    expect(word.className).toContain("text-warning");
    expect(view.getByTestId("rail-needs-you").textContent).toMatch(/Needs you\s*3/);
    expect(list.querySelectorAll('[data-rail-item="c:claude-work/outside-3"]')).toHaveLength(2);
  });

  it("a place folds when the user folds it, and stays folded", async () => {
    const view = board();
    const list = await view.findByTestId("place-board");
    const codex = await waitFor(() => list.querySelector<HTMLElement>('[data-order-head="ChatGPT|Codex"]')!);
    // A quiet place stays open in Your order.
    expect(within(codex).getByTestId("rail-fold").getAttribute("aria-expanded")).toBe("true");
    fireEvent.click(within(codex).getByTestId("rail-fold"));
    await waitFor(() => expect(list.querySelector("[data-conversation-id='outside-10']")).toBeNull());
    expect(railOrder().folded).toEqual(["place:ChatGPT|Codex"]);
  });
});
