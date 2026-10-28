/**
 * The 0.2 walk, F2 and F11 — the rail folded to icons. It said "All
 * clear" while the bell said four things needed you, with no session in
 * it: it knew only AI Juggler's own sessions and counted their needs
 * itself. Now its count is the backend's (D-49: the bell's, a sign-in
 * included), every open session shows as its app's or tool's mark, and it
 * is one stop on the Tab path, ↑↓ inside.
 */
import { needsFor } from "../test/needs-kit";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { FoldedRail, foldedOrder, needsYouWords } from "./RailFolded";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationsProvider } from "../lib/conversations";
import { resetSeen } from "../lib/seen";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { placesConversations } from "../preview/places";
import { previewNeedsOf } from "../preview/needs";
import { itemsOf, openNow } from "../lib/rail-items";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";

const NOW = new Date(2026, 9, 28, 10, 0, 0).getTime();
type Seams = { __AIJ_FIXTURE__?: unknown };
const seams = () => window as unknown as Seams;
const projects = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler" }] as ProjectInfo[];
const signedOut = { id: "claude-personal", label: "Personal", provider: "claude-code", login_state: "signed_out" } as AccountInfo;

beforeEach(() => {
  resetSeen();
  resetRailOrderCache();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  delete seams().__AIJ_FIXTURE__;
  vi.restoreAllMocks();
});

function folded(sessions: InstanceInfo[] = [], accounts: AccountInfo[] = [], openHome = vi.fn()) {
  const conversations = placesConversations(NOW);
  seams().__AIJ_FIXTURE__ = { conversations };
  needsFor({ sessions, conversations, projects, accounts });
  const view = render(
    <ActionHostProvider host={{ openHome }}>
      <ConversationsProvider>
        <FoldedRail sessions={sessions} projects={projects} now={NOW} selectedId={null} onSelect={() => {}} />
      </ConversationsProvider>
    </ActionHostProvider>,
  );
  return { view, conversations, openHome };
}

it("counts what the bell counts — a sign-in too — and never says All clear then", async () => {
  const { view, conversations, openHome } = folded([], [signedOut]);
  const expected = previewNeedsOf({ sessions: [], conversations, projects, accounts: [signedOut] }).length;
  expect(expected).toBeGreaterThan(2); // two sessions ask, and the sign-in
  const count = await view.findByTestId("rail-folded-needs-you");
  expect(count.textContent).toContain(String(expected));
  expect(count.getAttribute("aria-label")).toBe(`Needs you: ${needsYouWords(expected)}. Open Home`);
  expect(view.queryByTestId("rail-all-clear")).toBeNull();
  fireEvent.click(count);
  expect(openHome).toHaveBeenCalledTimes(1);
});

it("shows every open session another app runs, as its app's mark, with its state first in its name", async () => {
  const { view, conversations } = folded();
  const open = itemsOf({ sessions: [], conversations, projects, seen: {} }).filter(openNow);
  await waitFor(() => expect(view.container.querySelectorAll("button[data-rail-item]")).toHaveLength(open.length));
  expect(view.getAllByTestId("place-mark").length).toBe(open.length);
  const asks = view.container.querySelector<HTMLElement>("button[data-rail-item='c:claude-work/outside-3']")!;
  expect(asks.querySelector(".sr-only")!.textContent).toMatch(/^Asks you, Migrate the billing webhooks, /);
  expect(asks.querySelector("[data-testid='rail-folded-needs']")).not.toBeNull();
  // A background run and a closed one aren't open now.
  expect(view.container.querySelector("button[data-rail-item='c:claude-work/outside-7']")).toBeNull();
  expect(view.container.querySelector("button[data-rail-item='c:claude-work/outside-8']")).toBeNull();
});

it("says All clear only when nothing needs you and nothing is open", async () => {
  needsFor({ sessions: [] });
  const view = render(<FoldedRail sessions={[]} selectedId={null} onSelect={() => {}} />);
  expect(await view.findByTestId("rail-all-clear")).toBeTruthy();
  expect(view.queryByTestId("rail-folded-needs-you")).toBeNull();
});

it("is one stop on the Tab path; ↑ ↓ Home End move through it", async () => {
  const { view } = folded();
  await waitFor(() => expect(view.container.querySelectorAll("button[data-rail-item]").length).toBeGreaterThan(3));
  const stops = () => [...view.container.querySelectorAll<HTMLElement>("button")].filter((button) => button.tabIndex === 0);
  await waitFor(() => expect(stops()).toHaveLength(1));
  const first = stops()[0]!;
  expect(first.getAttribute("data-testid")).toBe("rail-folded-needs-you");
  first.focus();
  fireEvent.keyDown(first, { key: "ArrowDown" });
  const second = document.activeElement as HTMLElement;
  expect(second.getAttribute("data-rail-item")).not.toBeNull();
  // The one in the Tab path follows focus.
  await waitFor(() => expect(stops()).toEqual([second]));
  fireEvent.keyDown(second, { key: "End" });
  const buttons = [...view.container.querySelectorAll<HTMLElement>("button")];
  expect(document.activeElement).toBe(buttons[buttons.length - 1]);
  fireEvent.keyDown(document.activeElement!, { key: "Home" });
  expect(document.activeElement).toBe(first);
  fireEvent.keyDown(first, { key: "ArrowUp" });
  expect(document.activeElement).toBe(first);
});

it("keeps Open now's order: places as arranged, the user's pins first", () => {
  const conversations = placesConversations(NOW);
  const items = itemsOf({ sessions: [], conversations, projects, seen: {}, pins: new Set(["c:claude-work/outside-1"]) });
  const keys = foldedOrder(items, "yours", [], []).map((item) => item.key);
  // Each place's rows together.
  const places = foldedOrder(items, "yours", [], []).map((item) => item.place.key);
  expect(places.filter((place, index) => place !== places[index - 1])).toHaveLength(new Set(places).size);
  // A pin leads its place.
  const work = foldedOrder(items, "yours", [], []).filter((item) => item.place.key === items.find((entry) => entry.key === "c:claude-work/outside-1")!.place.key);
  expect(work[0]!.key).toBe("c:claude-work/outside-1");
  expect(keys).toContain("c:claude-work/outside-3");
});
