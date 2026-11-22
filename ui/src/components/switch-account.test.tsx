/**
 * D-143 (DESIGN-SYSTEM PAT-SW, VALIDATION N21): switching is the main
 * action, always one visible click away. The maintainer, 2026-11-21: on a
 * page whose account was out of room ("This week 0% left") the only way to
 * switch was a folded card's chevron, ⌘⇧H or a line in ⋯.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { JSX } from "react";
import * as notices from "../lib/ipc/notices";
import { SwitchAccountButton } from "./SwitchAccountButton";
import { useHandoverFocus } from "../lib/handover-focus";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationPage } from "./ConversationPage";
import { ConversationsProvider } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { resetConversationStore } from "../lib/conversation-store";
import { setSeam } from "../test/fixtures";

const NOW = Date.now();
const target = { session_id: null, conversation_id: null, account_id: "a1", project_id: null, page: null, task_id: null, pool_id: null };
const facts = { window: "weekly", resets_at_ms: NOW + 3 * 86_400_000, sessions: 0, version: null, channel: null, attempt: null, tries: null, account: "personal-max" };
const notice = (meter: number) => ({
  id: 1, kind: "quota", needs: true, title: "personal-max is out of room", detail: "0% of its week left", project_id: null, project: null, target, facts,
  action: "hand-over", meter, created_ms: NOW - 1, unread: true, subject: "n:1", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false,
});

function room(meter: number | null): void {
  setSeam("__AIJ_NOTICES__", { app: meter === null ? [] : [notice(meter)] });
  notices.resetNoticeStandIn();
}

beforeEach(() => room(null));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  resetConversationStore();
  Reflect.deleteProperty(window, "__AIJ_NOTICES__");
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
  notices.resetNoticeStandIn();
});

describe("Switch account… (D-143)", () => {
  it("is an accented action while its account has room, the filled primary when it's out or low", async () => {
    const view = render(<SwitchAccountButton switchKey="s-1" accountId="a1" />);
    const button = () => screen.getByTestId("switch-account");
    expect(button().textContent).toBe("Switch account…");
    expect(button().hasAttribute("data-primary")).toBe(false);
    expect(button().className).toContain("bg-accent-soft");
    view.unmount();
    for (const meter of [0, 6]) {
      room(meter);
      const again = render(<SwitchAccountButton switchKey="s-1" accountId="a1" />);
      await waitFor(() => expect(button().hasAttribute("data-primary")).toBe(true));
      expect(button().getAttribute("data-room")).toBe(meter === 0 ? "out" : "low");
      again.unmount();
    }
  });

  it("opens its session's switch list (the same as ⌘⇧H)", async () => {
    const opened = vi.fn();
    function List(): null {
      useHandoverFocus(opened, "s-1");
      return null;
    }
    render(
      <>
        <List />
        <SwitchAccountButton switchKey="s-1" accountId="a1" />
      </>,
    );
    await act(async () => void fireEvent.click(screen.getByTestId("switch-account")));
    expect(opened).toHaveBeenCalledTimes(1);
  });
});

function conversation(overrides: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id: "c", tool: "claude-code", account_id: "a1", account_label: "personal-max", project_id: "p1", cwd: "/w/app", title: "Fix the flaky test", git_branch: "main",
    started_ms: NOW - 3_600_000, last_activity_ms: NOW - 3_600_000, tokens: null, live: false, session_id: null, places: [], pid: null, state: "idle", viewed_ms: null,
    probably: false, background: false, inactive: true, subtask_count: null, ...overrides,
  };
}

function page(): JSX.Element {
  Object.assign(window, { __AIJ_FIXTURE__: { conversations: [conversation()] } });
  return (
    <ConversationsProvider>
      <ActionHostProvider host={{}}>
        <ConversationPage accountId="a1" id="c" now={NOW} />
      </ActionHostProvider>
    </ConversationsProvider>
  );
}

describe("a conversation's page (the maintainer's report: Claude app · personal-max, inactive, its week at 0%)", () => {
  it("shows Switch account… first, above Where it runs, outside the side column's scroll", async () => {
    render(page());
    const button = await screen.findByTestId("switch-account");
    const where = await screen.findByTestId("where-it-runs");
    expect(button.compareDocumentPosition(where) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    // Its own slot, the columns' first (above the main area when narrow,
    // atop the side column when wide), outside the column's scroll.
    const grid = screen.getByTestId("session-side").parentElement!.parentElement!;
    expect(grid.firstElementChild!.contains(button)).toBe(true);
    expect(screen.getByTestId("session-side").contains(button)).toBe(false);
  });

  // The 0.5.0 walk, F7: with the column's scroll bar shown, Switch
  // account… was 13 px wider than the cards under it.
  it("lines up with the cards under it while the column shows a scroll bar", async () => {
    const side = (el: HTMLElement) => el.dataset.testid === "session-side";
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) {
      return side(this) ? 340 : 0;
    });
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(function (this: HTMLElement) {
      return side(this) ? 327 : 0;
    });
    render(page());
    await screen.findByTestId("switch-account");
    await waitFor(() => expect(screen.getByTestId("session-lead").style.paddingRight).toBe("13px"));
  });

  it("out of room: Switch account… is the one filled primary; Continue in AI Juggler steps down", async () => {
    room(0);
    render(page());
    await waitFor(() => expect(screen.getByTestId("switch-account").hasAttribute("data-primary")).toBe(true));
    expect(screen.getByTestId("conversation-continue").hasAttribute("data-primary")).toBe(false);
    expect(document.querySelectorAll("[data-primary]")).toHaveLength(1);
  });

  it("with room: Continue in AI Juggler stays the primary; Switch account… is accented, still one click", async () => {
    render(page());
    const button = await screen.findByTestId("switch-account");
    expect(button.hasAttribute("data-primary")).toBe(false);
    expect(screen.getByTestId("conversation-continue").hasAttribute("data-primary")).toBe(true);
    expect(document.querySelectorAll("[data-primary]")).toHaveLength(1);
  });
});
