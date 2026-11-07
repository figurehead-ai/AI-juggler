/**
 * AJ-E3104 Req. 2 — every notice opens the right page, on the right tab:
 * a pool's message on its Messages (that thread), a task on Tasks, a
 * session working hard on its Processes, a restart on its History, a sign-in
 * on its App's Overview, an Account's renewal on its Settings.
 */
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../components/ItemMenu";
import type { NoticeInfo } from "./ipc/notices";
import * as messages from "./ipc/messages";
import * as subscriptions from "./subscriptions";
import * as toast from "../components/Toast";
import { OPEN_ACCOUNT } from "./accounts-page";
import { subscription } from "../test/fixtures";
import { useNoticeActions } from "./notice-actions";
import { rememberedTabId } from "../pages/registry";

vi.mock("./notices-store", () => ({ acted: () => Promise.resolve(true) }));

function notice(kind: string, action: string | null, target: Partial<NoticeInfo["target"]>): NoticeInfo {
  return {
    id: 1, kind, needs: true, title: kind, detail: "", project_id: null, project: null,
    target: { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null, ...target },
    facts: {} as NoticeInfo["facts"], action, meter: null, created_ms: 0, unread: false, subject: kind, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false,
  };
}

function actions(host: Record<string, (...args: never[]) => void>) {
  const wrapper = ({ children }: { children: ReactNode }) => <ActionHostProvider host={host}>{children}</ActionHostProvider>;
  return renderHook(() => useNoticeActions([]), { wrapper }).result.current;
}

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe("AJ-E3104 each notice opens its page on its tab", () => {
  it("a pool's message opens the pool on Messages, at its thread, not its sender's page", () => {
    const focus = vi.spyOn(messages, "focusThread");
    const openPool = vi.fn();
    const openSessionById = vi.fn();
    actions({ openPool, openSessionById }).primary(notice("message", "pool", { pool_id: "pool-1", session_id: "s-1", thread_id: "t-9" }));
    expect(openPool).toHaveBeenCalledWith("pool-1");
    expect(openSessionById).not.toHaveBeenCalled();
    expect(focus).toHaveBeenCalledWith("t-9");
    expect(rememberedTabId("pool")).toBe("messages");
  });

  it("a task opens the pool on Tasks, with or without the task's id", () => {
    const openPool = vi.fn();
    actions({ openPool }).primary(notice("task", "pool", { pool_id: "pool-1" }));
    expect(openPool).toHaveBeenCalledWith("pool-1");
    expect(rememberedTabId("pool")).toBe("tasks");
  });

  it("a session working hard opens on Processes; a restart on History; one asking on its body", () => {
    const openSessionById = vi.fn();
    const host = actions({ openSessionById });
    host.primary(notice("heavy", "open", { session_id: "s-1" }));
    expect(rememberedTabId("session")).toBe("processes");
    host.primary(notice("restarted", "see-why", { session_id: "s-2" }));
    expect(rememberedTabId("session")).toBe("history");
    host.primary(notice("asks", "open", { session_id: "s-3" }));
    expect(rememberedTabId("session")).toBe("terminal");
    expect(openSessionById).toHaveBeenCalledTimes(3);
  });

  it("another app's session not loaded yet opens by its ids, on Overview, not its App", () => {
    const openConversationById = vi.fn();
    const openAccountById = vi.fn();
    actions({ openConversationById, openAccountById }).primary(notice("asks", "open", { conversation_id: "c-1", account_id: "acc-1", project_id: "p-1" }));
    expect(openConversationById).toHaveBeenCalledWith("acc-1", "c-1", "p-1");
    expect(openAccountById).not.toHaveBeenCalled();
    expect(rememberedTabId("conversation")).toBe("overview");
  });

  it("a sign-in opens its App on Overview, where its sign-in card is", () => {
    const openAccountById = vi.fn();
    actions({ openAccountById }).primary(notice("signin", "sign-in", { account_id: "acc-1" }));
    expect(openAccountById).toHaveBeenCalledWith("acc-1");
    expect(rememberedTabId("app")).toBe("overview");
  });

  it("AJ-E2912: debug mode stopping by itself opens Settings at Diagnostics, where its log is", () => {
    const opened = vi.fn();
    window.addEventListener("aij:open-settings", opened);
    actions({}).primary(notice("debug", "open", { page: "diagnostics" }));
    window.removeEventListener("aij:open-settings", opened);
    expect(opened).toHaveBeenCalledTimes(1);
    expect(opened).toHaveBeenCalledWith(expect.objectContaining({ detail: expect.objectContaining({ section: "diagnostics" }) }));
  });

  it("an Account's renewal opens its page on Settings; its limits on Overview (AJ-E3104 R2)", () => {
    vi.spyOn(subscriptions, "cachedSubscriptions").mockReturnValue([subscription({ folder: "claude-work", account_id: "acct:anthropic:work" })]);
    const opened = vi.fn();
    const listen = (event: Event) => opened(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener(OPEN_ACCOUNT, listen);
    const openAccountById = vi.fn();
    const host = actions({ openAccountById });
    host.primary(notice("renewal", "open", { account_id: "claude-work" }));
    expect(rememberedTabId("subscription")).toBe("settings");
    host.primary(notice("limit", "open", { account_id: "claude-work" }));
    expect(rememberedTabId("subscription")).toBe("overview");
    window.removeEventListener(OPEN_ACCOUNT, listen);
    expect(opened.mock.calls).toEqual([[{ id: "acct:anthropic:work" }], [{ id: "acct:anthropic:work" }]]);
    // Its sign-in known: the Account, never the App.
    expect(openAccountById).not.toHaveBeenCalled();
  });

  it("a notice whose subject is gone says so, and opens nothing (AJ-E3104 R2)", () => {
    const shown = vi.spyOn(toast, "showToast");
    const openSessionById = vi.fn();
    const openPool = vi.fn();
    actions({ openSessionById, openPool }).primary(notice("asks", "open", {}));
    expect(shown).toHaveBeenCalledWith(expect.objectContaining({ message: "Nothing to open for this one" }));
    expect(openSessionById).not.toHaveBeenCalled();
    expect(openPool).not.toHaveBeenCalled();
  });
});
