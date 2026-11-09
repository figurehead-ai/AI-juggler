/**
 * rc21 vitest — aij-test's copy `<id>-fork-…` (an id an older AI Juggler
 * made): its ended session's Resume was greyed with no words (a native
 * title on a `disabled` button, which gets no pointer and no focus). Now
 * every door that resumes it says its tool's words, from the backend's
 * one check: Resume on hover and keyboard focus and as its description,
 * with Open the original beside it; Copy the resume command hands out no
 * command; the menus' and ⌘K's doors are greyed with the same words.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ResumeButton } from "./ResumeButton";
import { AppActions } from "../../components/AppActions";
import { useActionHost } from "../../components/ItemMenu";
import { ToastHost, resetToasts } from "../../components/Toast";
import { resolveActions } from "../../lib/actions";
import * as conversations from "../../lib/ipc/conversations";
import * as sessions from "../../lib/ipc/sessions";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ResumeRefusalInfo } from "../../types/generated/ResumeRefusalInfo";
import { placesConversations } from "../../preview/places";
import { clearSeams, instance, seamCalls } from "../../test/fixtures";

const ORIGINAL = "a44c2a28-4542-45c3-a4bb-dadfce0e8b3a";
const COPY = `${ORIGINAL}-fork-1a1bf34281e-0`;
const WHY =
  "Claude Code can't open this copy by its id (an older AI Juggler made it): continue the conversation it was copied from, or switch that one again";
const known: ResumeRefusalInfo = { why: WHY, original: { account_id: "acc-lab", account_label: "Lab", id: ORIGINAL }, find: null };
const unknown: ResumeRefusalInfo = { why: WHY, original: null, find: "Find the conversation it was copied from in History: it has the same title, on the App it was switched from." };

const session = (refusal: ResumeRefusalInfo | null = known): InstanceInfo =>
  instance({
    id: "sess-old-copy",
    provider: "claude-code",
    status: "finished",
    conversation_id: COPY,
    account_id: "acc-aij",
    lifecycle: "active",
    resume_refusal: refusal ?? undefined,
  });

const conversation = (refusal: ResumeRefusalInfo | null = known): ConversationInfo => ({
  ...placesConversations(Date.now())[0]!,
  id: COPY,
  live: false,
  resume_refusal: refusal ?? undefined,
});

afterEach(() => {
  cleanup();
  resetToasts();
  vi.restoreAllMocks();
  clearSeams("__AIJ_SESSION_RESUME__");
});

describe("rc21 — a greyed Resume says why", () => {
  it("carries its tool's words as its description and in a tip on hover and on keyboard focus; a press starts nothing", async () => {
    render(<ResumeButton session={session()} openIn={null} onOpen={vi.fn()} />);
    const resume = screen.getByTestId("header-resume");
    expect(resume.getAttribute("aria-disabled")).toBe("true");
    expect(resume.getAttribute("aria-description")).toBe(WHY);
    expect(resume.hasAttribute("disabled"), "a disabled button gets no hover and no focus").toBe(false);
    expect(resume.hasAttribute("title"), "no native title: the app's view never showed it").toBe(false);
    expect(screen.queryByRole("tooltip")).toBeNull();
    // Hover.
    await act(async () => {
      fireEvent.pointerEnter(resume);
    });
    expect(screen.getByRole("tooltip").textContent).toBe(WHY);
    expect(resume.getAttribute("aria-describedby")).toBe(screen.getByRole("tooltip").id);
    await act(async () => {
      fireEvent.pointerLeave(resume);
    });
    expect(screen.queryByRole("tooltip")).toBeNull();
    // Keyboard focus: a tab stop still.
    expect(resume.tabIndex).toBe(0);
    fireEvent.keyDown(document.body, { key: "Tab" });
    await act(async () => {
      resume.focus();
    });
    expect(screen.getByRole("tooltip").textContent).toBe(WHY);
    await act(async () => {
      fireEvent.keyDown(window, { key: "Escape" });
    });
    expect(screen.queryByRole("tooltip")).toBeNull();
    // A press says why again and starts nothing.
    await act(async () => {
      fireEvent.click(resume);
    });
    expect(screen.getByRole("tooltip").textContent).toBe(WHY);
    expect(seamCalls("__AIJ_SESSION_RESUME__")).toEqual([]);
  });

  it("offers Open the original, the conversation the switch copied it from", () => {
    const onOpenOriginal = vi.fn();
    render(<ResumeButton session={session()} openIn={null} onOpenOriginal={onOpenOriginal} />);
    const open = screen.getByTestId("open-original");
    expect(open.textContent).toBe("Open the original");
    expect(open.getAttribute("aria-description")).toBe("The conversation it was copied from on Lab");
    fireEvent.click(open);
    expect(onOpenOriginal).toHaveBeenCalledWith("acc-lab", ORIGINAL, null);
  });

  it("says how to find the original when AI Juggler doesn't know it", () => {
    render(<ResumeButton session={session(unknown)} openIn={null} onOpenOriginal={vi.fn()} />);
    expect(screen.getByTestId("header-resume").getAttribute("aria-description")).toBe(`${WHY}. ${unknown.find}`);
    expect(screen.queryByTestId("open-original")).toBeNull();
  });

  it("a copy its tool opens is Resume as before", () => {
    render(<ResumeButton session={session(null)} openIn={null} />);
    const resume = screen.getByTestId("header-resume");
    expect(resume.hasAttribute("aria-disabled")).toBe(false);
    expect(screen.queryByTestId("open-original")).toBeNull();
  });
});

describe("rc21 — no resume command its tool won't open", () => {
  function Doors({ row, talk }: { row: InstanceInfo; talk: ConversationInfo }) {
    const host = useActionHost();
    return (
      <>
        <button type="button" onClick={() => void host.copyResumeCommand?.(row)}>
          Copy session
        </button>
        <button type="button" onClick={() => void host.copyConversationResume?.(talk)}>
          Copy conversation
        </button>
        <button type="button" onClick={() => void host.continueConversation?.(talk)}>
          Continue
        </button>
      </>
    );
  }

  it("Copy the resume command copies nothing and says why; Continue in AI Juggler starts nothing", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    const sessionCommand = vi.spyOn(sessions, "sessionsResumeCommand");
    const conversationCommand = vi.spyOn(conversations, "conversationsResumeCommand");
    const carry = vi.spyOn(conversations, "conversationsContinue");
    const none = () => {};
    render(
      <AppActions selectConversation={none} selectSession={none} selectProject={none} selectAccount={none} selectTool={none} openToolSettings={none} selectPool={none} openAccounts={none} openHome={none} stopSession={none} newSessionIn={none} refresh={none}>
        <Doors row={session()} talk={conversation()} />
        <ToastHost />
      </AppActions>,
    );
    for (const name of ["Copy session", "Copy conversation"]) {
      resetToasts();
      fireEvent.click(screen.getByRole("button", { name }));
      await waitFor(() => expect(screen.getByTestId("toast-stack").textContent).toContain("No resume command to copy"));
      expect(screen.getByTestId("toast-stack").textContent).toContain("can't open this copy by its id");
    }
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(screen.getByTestId("toast-stack").textContent).toContain("Couldn't continue it here"));
    expect(writeText).not.toHaveBeenCalled();
    expect(sessionCommand).not.toHaveBeenCalled();
    expect(conversationCommand).not.toHaveBeenCalled();
    expect(carry).not.toHaveBeenCalled();
  });

  it("the menus' and ⌘K's doors are greyed with the same words, and run nothing", () => {
    const host = { copyResumeCommand: vi.fn(), copyConversationResume: vi.fn(), continueConversation: vi.fn() };
    const copySession = resolveActions("session", session(), host).find((a) => a.id === "copy-resume")!;
    expect(copySession.reason).toBe(WHY);
    void copySession.run();
    expect(host.copyResumeCommand).not.toHaveBeenCalled();
    const talk = resolveActions("conversation", conversation(), host);
    for (const id of ["copy-resume", "continue"]) {
      const door = talk.find((a) => a.id === id)!;
      expect(door.reason, id).toBe(WHY);
      void door.run();
    }
    expect(host.copyConversationResume).not.toHaveBeenCalled();
    expect(host.continueConversation).not.toHaveBeenCalled();
    // One its tool opens: as before.
    expect(resolveActions("session", session(null), host).find((a) => a.id === "copy-resume")!.reason).toBeUndefined();
  });
});
