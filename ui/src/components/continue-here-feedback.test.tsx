/**
 * AJ-E3006 R2 (the rc19 walk): Continue in AI Juggler says a new process
 * resumes the conversation here once it has started, and the reason in
 * words when it was refused (nothing started).
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AppActions } from "./AppActions";
import { useActionHost } from "./ItemMenu";
import { ToastHost, resetToasts } from "./Toast";
import * as conversations from "../lib/ipc/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { CONTINUED_HERE } from "../lib/pools";
import { placesConversations } from "../preview/places";

afterEach(() => {
  cleanup();
  resetToasts();
  vi.restoreAllMocks();
});

function ContinueButton({ conversation }: { conversation: ConversationInfo }) {
  const host = useActionHost();
  return (
    <button type="button" onClick={() => void host.continueConversation?.(conversation)}>
      Continue in AI Juggler
    </button>
  );
}

function renderIt(selectSession: (id: string) => void) {
  const none = () => {};
  render(
    <AppActions selectConversation={none} selectSession={selectSession} selectProject={none} selectAccount={none} selectTool={none} openToolSettings={none} selectPool={none} openAccounts={none} openHome={none} stopSession={none} newSessionIn={none} refresh={none}>
      <ContinueButton conversation={placesConversations(Date.now())[0]!} />
      <ToastHost />
    </AppActions>,
  );
}

it("says a new process resumes it here, and opens its session", async () => {
  vi.spyOn(conversations, "conversationsContinue").mockResolvedValue({ session_id: "sess-new", pid: 42 });
  const selectSession = vi.fn();
  renderIt(selectSession);
  fireEvent.click(screen.getByRole("button", { name: "Continue in AI Juggler" }));
  await waitFor(() => expect(screen.getByTestId("toast-stack").textContent).toContain(CONTINUED_HERE));
  expect(CONTINUED_HERE).toMatch(/new process/);
  expect(selectSession).toHaveBeenCalledWith("sess-new");
});

it("a refusal says why, and nothing opens", async () => {
  vi.spyOn(conversations, "conversationsContinue").mockRejectedValue(new Error("Claude Code can't open this copy by its id"));
  const selectSession = vi.fn();
  renderIt(selectSession);
  fireEvent.click(screen.getByRole("button", { name: "Continue in AI Juggler" }));
  await waitFor(() => expect(screen.getByTestId("toast-stack").textContent).toContain("Couldn't continue it here"));
  expect(screen.getByTestId("toast-stack").textContent).toContain("can't open this copy");
  expect(selectSession).not.toHaveBeenCalled();
});
