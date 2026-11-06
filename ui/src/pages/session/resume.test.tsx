/**
 * AJ-E2908 vitest — an ended session's page: Resume (the same
 * conversation) with the reason when it can't, and History's Replay,
 * Archive and Copy the conversation's id in its ⋯, never on a live one.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ResumeButton, resumeBlocked } from "./ResumeButton";
import { resolveActions } from "../../lib/actions";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

const session = (over: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({
    id: "s-1",
    status: "finished",
    conversation_id: "conv-1",
    account_id: "acc-x",
    lifecycle: "active",
    ...over,
  }) as InstanceInfo;

const account = (state: string) => ({ id: "acc-x", label: "Work", login_state: state }) as AccountInfo;

afterEach(() => {
  cleanup();
  delete (window as unknown as { __AIJ_SESSION_RESUME__?: string[] }).__AIJ_SESSION_RESUME__;
});

describe("AJ-E2908 — an ended session's page", () => {
  it("says why Resume can't run", () => {
    expect(resumeBlocked({ session: session({ conversation_id: null }), account: null, out: null, openIn: null })).toBe("It has no conversation to resume yet");
    expect(resumeBlocked({ session: session(), account: account("signed_out"), out: null, openIn: null })).toBe("Sign in to Work first");
    expect(resumeBlocked({ session: session(), account: account("signed_in"), out: "Work is out of room until 17:00", openIn: null })).toBe("Work is out of room until 17:00");
    expect(resumeBlocked({ session: session(), account: null, out: null, openIn: "another app" })).toContain("open in another app now");
    expect(resumeBlocked({ session: session(), account: account("signed_in"), out: null, openIn: null })).toBeNull();
  });

  it("resumes the same conversation and opens the new session", async () => {
    const onOpen = vi.fn();
    render(<ResumeButton session={session()} openIn={null} onOpen={onOpen} />);
    const button = screen.getByTestId("header-resume");
    expect(button.getAttribute("title")).toBe("Resume — same session, same account.");
    await act(async () => {
      fireEvent.click(button);
    });
    expect((window as unknown as { __AIJ_SESSION_RESUME__?: string[] }).__AIJ_SESSION_RESUME__).toEqual(["s-1"]);
    expect(onOpen).toHaveBeenCalledWith("sess-resumed-s-1");
  });

  it("is off, with its reason, without a conversation", () => {
    render(<ResumeButton session={session({ conversation_id: null })} openIn={null} />);
    expect((screen.getByTestId("header-resume") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByTestId("header-resume-why").textContent).toBe("It has no conversation to resume yet");
  });

  it("adds Replay, Archive and Copy the conversation's id to an ended session's menu only", () => {
    const host = { replay: vi.fn(), archive: vi.fn(), unarchive: vi.fn(), isArchived: () => false, copyConversationId: vi.fn() };
    const ended = resolveActions("session", session(), host).map((a) => a.id);
    expect(ended).toEqual(expect.arrayContaining(["replay", "archive", "copy-id"]));
    const live = resolveActions("session", session({ status: "running" }), host).map((a) => a.id);
    expect(live).not.toEqual(expect.arrayContaining(["replay"]));
    expect(live).not.toContain("archive");
    expect(live).not.toContain("copy-id");
    const archived = resolveActions("session", session({ lifecycle: "archived" }), { ...host, isArchived: () => true });
    expect(archived.find((a) => a.id === "archive")?.label).toBe("Unarchive");
  });
});
