/**
 * 0.1.5 review — the conversations list is the backend's working set; a
 * page open for an older conversation (from History) still finds it: the
 * provider reads it by its id and holds it while it's wanted.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, waitFor } from "@testing-library/react";
import type { ConversationInfo } from "./ipc/conversations";

const old = { id: "c-old", account_id: "acc", title: "Last spring's refactor", live: false } as ConversationInfo;
const got: string[] = [];
vi.mock("./ipc/conversations", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/conversations")>()),
  conversationsList: async () => [{ id: "c-new", account_id: "acc", live: true }],
  conversationsGet: async (accountId: string, id: string) => {
    got.push(`${accountId}/${id}`);
    return id === old.id ? old : null;
  },
}));

const { ConversationsProvider, useConversations } = await import("./conversations");

afterEach(() => {
  cleanup();
  got.length = 0;
});

function Page({ id }: { id: string }) {
  const { all } = useConversations();
  const found = all.find((c) => c.account_id === "acc" && c.id === id);
  return <p data-testid="page">{found ? (found.title ?? found.id) : "isn't there"}</p>;
}

describe("a conversation past the working set", () => {
  it("is read by its id while its page is open, and leaves with it", async () => {
    const view = render(
      <ConversationsProvider want={{ accountId: "acc", id: "c-old" }}>
        <Page id="c-old" />
      </ConversationsProvider>,
    );
    await waitFor(() => expect(view.getByTestId("page").textContent).toBe("Last spring's refactor"));
    expect(got).toContain("acc/c-old");
    view.rerender(
      <ConversationsProvider want={null}>
        <Page id="c-old" />
      </ConversationsProvider>,
    );
    expect(view.getByTestId("page").textContent).toBe("isn't there");
  });

  it("asks for nothing more once it's in the list", async () => {
    const view = render(
      <ConversationsProvider want={{ accountId: "acc", id: "c-new" }}>
        <Page id="c-new" />
      </ConversationsProvider>,
    );
    await waitFor(() => expect(view.getByTestId("page").textContent).toBe("c-new"));
    const asked = got.length;
    view.rerender(
      <ConversationsProvider want={{ accountId: "acc", id: "c-new" }}>
        <Page id="c-new" />
      </ConversationsProvider>,
    );
    expect(got.length).toBe(asked);
  });
});
