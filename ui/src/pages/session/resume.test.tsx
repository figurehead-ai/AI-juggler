/**
 * AJ-E2908 vitest — an ended session's page: Resume (the same
 * conversation) with the reason when it can't, and History's Replay,
 * Archive and Copy the conversation's id in its ⋯, never on a live one.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ResumeButton, resumeBlocked } from "./ResumeButton";
import { resolveActions } from "../../lib/actions";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { clearSeams, seamCalls, setSeam } from "../../test/fixtures";

const session = (over: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({
    id: "s-1",
    provider: "claude-code",
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

  it("the RC walk: a tool with a run-without-a-terminal runtime offers Run in AI Juggler, the same conversation", async () => {
    const onOpen = vi.fn();
    render(<ResumeButton session={session()} openIn={null} onOpen={onOpen} />);
    const here = screen.getByTestId("header-run-here");
    expect(here.textContent).toBe("Run in AI Juggler");
    await act(async () => {
      fireEvent.click(here);
    });
    expect(seamCalls("__AIJ_RUNTIME_CALLS__")).toContain("handover:s-1:driven");
    expect(onOpen).toHaveBeenCalledWith("s-1");
    // None without a conversation, or for a tool with one runtime.
    cleanup();
    render(<ResumeButton session={session({ conversation_id: null })} openIn={null} />);
    expect(screen.queryByTestId("header-run-here")).toBeNull();
    cleanup();
    render(<ResumeButton session={session({ provider: "acme-cli" })} openIn={null} />);
    expect(screen.queryByTestId("header-run-here")).toBeNull();
  });

  it("R2: whether its tool can resume is the tool's own spec, never a branch here", () => {
    // A tool whose adapter gives no resume arguments (an unknown one gets
    // the neutral spec, which claims nothing).
    render(<ResumeButton session={session({ provider: "acme-cli" })} openIn={null} />);
    expect(screen.getByTestId("header-resume").getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByTestId("header-resume").getAttribute("aria-description")).toBe("acme-cli can't resume a conversation from AI Juggler");
  });

  it("is off, with its reason, without a conversation", () => {
    render(<ResumeButton session={session({ conversation_id: null })} openIn={null} />);
    expect(screen.getByTestId("header-resume").getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByTestId("header-resume").getAttribute("aria-description")).toBe("It has no conversation to resume yet");
  });

  it("adds Replay, Archive and Copy the conversation's id to an ended session's menu; a running one's Archive is greyed with why", () => {
    const host = { replay: vi.fn(), archive: vi.fn(), unarchive: vi.fn(), isArchived: () => false, copyConversationId: vi.fn() };
    const ended = resolveActions("session", session(), host).map((a) => a.id);
    expect(ended).toEqual(expect.arrayContaining(["replay", "archive", "copy-id"]));
    const live = resolveActions("session", session({ status: "running" }), host).map((a) => a.id);
    expect(live).not.toEqual(expect.arrayContaining(["replay"]));
    // AJ-E2910 R4: Archive stays, greyed with why, so the e key's silence
    // on a running session is explained.
    const archiveLive = resolveActions("session", session({ status: "running" }), host).find((a) => a.id === "archive");
    expect(archiveLive?.reason).toBe("It's still running: stop it first, then archive it");
    expect(live).not.toContain("copy-id");
    const archived = resolveActions("session", session({ lifecycle: "archived" }), { ...host, isArchived: () => true });
    expect(archived.find((a) => a.id === "archive")?.label).toBe("Unarchive");
  });
});

describe("AJ-E2908 R3 — out of room, Continue on another account… beside Resume", () => {
  afterEach(() => {
    clearSeams("__AIJ_QUOTA_DISPLAY__");
  });

  it("says the account is out of room on Resume, and offers Continue on another account…", async () => {
    const { quotaDisplay, windowDisplay } = await import("../../lib/wire");
    setSeam("__AIJ_QUOTA_DISPLAY__", [{ ...quotaDisplay("acc-x"), five_hour: windowDisplay("tracking", { percentage: 100, resets_at_ms: Date.now() + 2 * 3_600_000 }) }]);
    const onContinue = vi.fn();
    render(<ResumeButton session={session()} openIn={null} onContinue={onContinue} />);
    const go = await screen.findByTestId("header-resume-continue");
    expect(go.textContent).toBe("Continue on another account…");
    expect(screen.getByTestId("header-resume").getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByTestId("header-resume").getAttribute("aria-description")).toMatch(/out of room/);
    fireEvent.click(go);
    expect(onContinue).toHaveBeenCalledTimes(1);
  });

  it("with room, no Continue beside it", () => {
    render(<ResumeButton session={session()} openIn={null} onContinue={vi.fn()} />);
    expect(screen.queryByTestId("header-resume-continue")).toBeNull();
  });
});

describe("AJ-E2908 R4 and R5 — the session page's ⋯", () => {
  const ended = session({ id: "s-9", conversation_id: "conv-tool-42", label: "Fix billing" });

  async function openMenu() {
    const { InspectorHeaderMenu } = await import("../../components/InspectorHeaderMenu");
    const { ToastHost } = await import("../../components/Toast");
    render(
      <>
        <InspectorHeaderMenu kind="instance" data={{ instance: ended }} onChanged={() => {}} />
        <ToastHost />
      </>,
    );
    fireEvent.click(screen.getByTestId("header-menu"));
    return screen.getByRole("menu");
  }
  const labels = (list: HTMLElement) => within(list).getAllByRole("menuitem").map((item) => item.querySelector("[data-label]")?.textContent ?? item.textContent);

  const itemOf = (list: HTMLElement, label: string) => {
    const found = within(list).getAllByRole("menuitem").find((item) => (item.querySelector("[data-label]")?.textContent ?? "") === label);
    if (!found) throw new Error(`no ${label}`);
    return found;
  };

  afterEach(async () => {
    (await import("../../components/Toast")).resetToasts();
    clearSeams("__AIJ_HISTORY_CALLS__");
  });

  it("R4: an ended session's ⋯ has Replay, Archive and Copy the conversation's id; Archive's toast offers Undo", async () => {
    const open = await openMenu();
    expect(labels(open)).toEqual(expect.arrayContaining(["Replay", "Archive", "Copy the conversation's id"]));
    await act(async () => {
      fireEvent.click(itemOf(open, "Archive"));
    });
    const toast = await screen.findByTestId("toast");
    expect(toast.textContent).toContain("archived — nothing deleted.");
    const calls = () => seamCalls("__AIJ_HISTORY_CALLS__");
    expect(calls()).toEqual(["archive:s-9"]);
    await act(async () => {
      fireEvent.click(within(toast).getByRole("button", { name: /Undo/ }));
    });
    await waitFor(() => expect(calls()).toEqual(["archive:s-9", "unarchive:s-9"]));
  });

  it("R5: Copy the conversation's id writes the tool's own id and says “Copied.”", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    const open = await openMenu();
    await act(async () => {
      fireEvent.click(itemOf(open, "Copy the conversation's id"));
    });
    expect(writeText).toHaveBeenCalledWith("conv-tool-42");
    expect((await screen.findByTestId("toast")).textContent).toContain("Copied.");
  });
});
