/**
 * A first start's conversation list: one quiet line while it's built,
 * gone by itself once it is (and never asked about again in that run).
 */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { JSX } from "react";
import { ConversationsProvider, useConversations } from "../lib/conversations";
import { ListBuilding } from "./ListBuilding";

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_LIST_BUILDING__");
});

function Again(): JSX.Element {
  const { refresh } = useConversations();
  return (
    <button type="button" onClick={refresh}>
      again
    </button>
  );
}

it("says the list is building, then clears itself", async () => {
  Reflect.set(window, "__AIJ_LIST_BUILDING__", true);
  const view = render(
    <ConversationsProvider>
      <ListBuilding place="home" />
      <ListBuilding place="rail" />
      <Again />
    </ConversationsProvider>,
  );
  expect((await view.findByTestId("list-building-home")).textContent).toBe("Building the conversation list…");
  expect(view.getByTestId("list-building-rail")).toBeTruthy();
  Reflect.set(window, "__AIJ_LIST_BUILDING__", false);
  act(() => view.getByText("again").click());
  await waitFor(() => expect(view.queryByTestId("list-building-home")).toBeNull());
  expect(view.queryByTestId("list-building-rail")).toBeNull();
});
