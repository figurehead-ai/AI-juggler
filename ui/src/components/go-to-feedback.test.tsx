/**
 * AJ-E2102 R2: Go to it says "Going to it…" at once, before the backend
 * answers, then what really happened, in the same toast.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AppActions } from "./AppActions";
import { useActionHost } from "./ItemMenu";
import { ToastHost, resetToasts } from "./Toast";
import * as conversations from "../lib/ipc/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { GoToOutcomeInfo } from "../types/generated/GoToOutcomeInfo";
import { placesConversations } from "../preview/places";

afterEach(() => {
  cleanup();
  resetToasts();
  vi.restoreAllMocks();
});

function GoButton({ conversation }: { conversation: ConversationInfo }) {
  const host = useActionHost();
  return (
    <button type="button" onClick={() => void host.goToConversation?.(conversation)}>
      Go to it
    </button>
  );
}

it("says Going to it… at once, then the outcome in the same toast", async () => {
  let answer: (outcome: GoToOutcomeInfo) => void = () => {};
  vi.spyOn(conversations, "conversationsGoTo").mockImplementation(() => new Promise((resolve) => (answer = resolve)));
  const none = () => {};
  render(
    <AppActions selectConversation={none} selectSession={none} selectProject={none} selectAccount={none} selectTool={none} openToolSettings={none} selectPool={none} openAccounts={none} openHome={none} stopSession={none} newSessionIn={none} refresh={none}>
      <GoButton conversation={placesConversations(Date.now())[0]!} />
      <ToastHost />
    </AppActions>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Go to it" }));
  // Before the backend answers: the feedback is already there.
  expect(screen.getByTestId("toast-stack").textContent).toContain("Going to it…");
  await act(async () => answer({ outcome: "Brought Claude · Work forward on the session", reached: "thread" }));
  await waitFor(() => expect(screen.getByTestId("toast-stack").textContent).toContain("Brought Claude · Work forward on the session"));
  expect(screen.getByTestId("toast-stack").textContent).not.toContain("Going to it…");
});
