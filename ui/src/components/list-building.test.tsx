/**
 * A first start's conversation list: one quiet line while it's built,
 * how far it is, gone by itself once it is (and never asked about again
 * in that run); a failed build says so with a Try again.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { JSX } from "react";
import type { ListBuildInfo } from "../types/generated/ListBuildInfo";
import { ConversationsProvider, useConversations } from "../lib/conversations";
import { ListBuilding } from "./ListBuilding";

const control = vi.hoisted(() => ({
  building: null as null | (() => Promise<ListBuildInfo>),
  again: vi.fn(async () => {}),
}));

vi.mock("../lib/ipc/conversations", async (original) => {
  const real = await original<typeof import("../lib/ipc/conversations")>();
  return {
    ...real,
    conversationsBuilding: () => (control.building ? control.building() : real.conversationsBuilding()),
    conversationsBuildAgain: () => control.again(),
  };
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  control.building = null;
  control.again.mockClear();
  Reflect.deleteProperty(window, "__AIJ_LIST_BUILDING__");
});

const at = (info: Partial<ListBuildInfo>): ListBuildInfo => ({ building: true, read: 0, total: 0, failed: false, ...info });

function Again(): JSX.Element {
  const { refresh } = useConversations();
  return (
    <button type="button" onClick={refresh}>
      again
    </button>
  );
}

function shown(): ReturnType<typeof render> {
  return render(
    <ConversationsProvider>
      <ListBuilding place="home" />
      <ListBuilding place="rail" />
      <Again />
    </ConversationsProvider>,
  );
}

it("says the list is building, then clears itself", async () => {
  Reflect.set(window, "__AIJ_LIST_BUILDING__", true);
  const view = shown();
  expect((await view.findByTestId("list-building-home")).textContent).toBe("Building the conversation list…");
  expect(view.getByTestId("list-building-rail")).toBeTruthy();
  Reflect.set(window, "__AIJ_LIST_BUILDING__", false);
  act(() => view.getByText("again").click());
  await waitFor(() => expect(view.queryByTestId("list-building-home")).toBeNull());
  expect(view.queryByTestId("list-building-rail")).toBeNull();
});

it("says how far the build is", async () => {
  Reflect.set(window, "__AIJ_LIST_BUILDING__", at({ read: 120, total: 1000 }));
  const view = shown();
  expect((await view.findByTestId("list-building-home")).textContent).toBe("Reading conversations: 120 of 1,000");
});

// rc24 walk K1: the build ended with nothing new to tell, so no event came
// and the line stayed for good. It asks again by itself while it shows.
it("clears when the build ends even if nothing tells it", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  Reflect.set(window, "__AIJ_LIST_BUILDING__", true);
  const view = shown();
  await view.findByTestId("list-building-home");
  Reflect.set(window, "__AIJ_LIST_BUILDING__", false);
  await act(() => vi.advanceTimersByTimeAsync(2_500));
  await waitFor(() => expect(view.queryByTestId("list-building-home")).toBeNull());
});

// Under heavy swapping the answers can come back out of order: an older
// "still building" landing after "built" turned the line back on for good.
it("an older answer never turns the line back on", async () => {
  const answers: Array<(info: ListBuildInfo) => void> = [];
  control.building = () => new Promise((resolve) => answers.push(resolve));
  const view = shown();
  await waitFor(() => expect(answers.length).toBe(1));
  act(() => view.getByText("again").click());
  await waitFor(() => expect(answers.length).toBe(2));
  await act(async () => answers[1]!(at({ building: false, read: 3, total: 3 })));
  await act(async () => answers[0]!(at({ read: 1, total: 3 })));
  expect(view.queryByTestId("list-building-home")).toBeNull();
});

it("a failed build says so, and Try again asks again", async () => {
  Reflect.set(window, "__AIJ_LIST_BUILDING__", at({ failed: true }));
  const view = shown();
  const home = await view.findByTestId("list-building-home");
  expect(home.textContent).toContain("Couldn't read the conversation list");
  Reflect.set(window, "__AIJ_LIST_BUILDING__", false);
  fireEvent.click(view.getByTestId("list-building-home-again"));
  expect(control.again).toHaveBeenCalledTimes(1);
  await waitFor(() => expect(view.queryByTestId("list-building-home")).toBeNull());
});
