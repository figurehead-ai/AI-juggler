/**
 * The 0.2 walk, F8 and F16 — the switch from anywhere a session is:
 * "Continue on another account…" on another app's session's row menu (and
 * so ⌘K, the same registry) with ⌘⇧H; ⌘⇧H takes the open session's switch
 * list, whatever its kind, or says why not; an out-of-room banner
 * continues the account's open session, never its account page.
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { resolveActions, type ActionHost } from "./lib/actions";
import { focusHandover, focusHandoverSoon, NO_SESSION_OPEN, openOnAccount, useHandoverFocus } from "./lib/handover-focus";
import { useAppKeys, type AppKeyActions } from "./lib/app-keys";
import { resetToasts, ToastHost } from "./components/Toast";
import type { ConversationInfo } from "./lib/ipc/conversations";
import type { InstanceInfo } from "./lib/bridge";

afterEach(() => {
  cleanup();
  resetToasts();
  vi.useRealTimers();
});

const conversation = (overrides: Partial<ConversationInfo> = {}): ConversationInfo =>
  ({ id: "c-1", tool: "claude-code", account_id: "f", account_label: "Account F", project_id: null, live: true, session_id: null, started_ms: 1, last_activity_ms: 10, places: [] , ...overrides }) as unknown as ConversationInfo;

describe("F8 the row menu of another app's session", () => {
  it("offers Continue on another account… with ⌘⇧H, before Continue in AI Juggler", () => {
    const host: ActionHost = { continueConversationElsewhere: vi.fn(), continueConversation: vi.fn() };
    const actions = resolveActions("conversation", conversation(), host);
    const ids = actions.map((action) => action.id);
    expect(ids.indexOf("continue-elsewhere")).toBeLessThan(ids.indexOf("continue"));
    const action = actions.find((entry) => entry.id === "continue-elsewhere")!;
    expect(action.label).toBe("Continue on another account…");
    expect(action.shortcut).toBe("⌘⇧H");
    expect(action.reason).toBeUndefined();
    void action.run();
    expect(host.continueConversationElsewhere).toHaveBeenCalledTimes(1);
  });
});

function List({ took }: { took: () => void }) {
  useHandoverFocus(took);
  return <p>list</p>;
}

function Keys() {
  const noop = () => {};
  useAppKeys({ openShortcuts: noop, togglePalette: noop, toggleNotifications: noop, newSession: noop, newProject: noop, toggleRail: noop, toggleInspector: noop, cycleDensity: noop, goHome: noop, goTo: noop, openSettings: noop, onCleanup: () => false, focusCleanupFilter: noop, stopSelected: noop } satisfies AppKeyActions);
  return <ToastHost />;
}

const press = () => act(() => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "h", code: "KeyH", metaKey: true, shiftKey: true, bubbles: true })));

describe("F8 ⌘⇧H", () => {
  it("with a session's page open, its switch list takes it", () => {
    const took = vi.fn();
    render(
      <>
        <Keys />
        <List took={took} />
      </>,
    );
    press();
    expect(took).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(NO_SESSION_OPEN)).toBeNull();
  });

  it("with no session's page open, says why instead of nothing", () => {
    render(<Keys />);
    press();
    expect(screen.getAllByText(NO_SESSION_OPEN).length).toBeGreaterThan(0);
    expect(focusHandover()).toBe(false);
  });

  it("a page that opens just after the ask takes it as it mounts (the row menu opens the page first)", async () => {
    vi.useFakeTimers();
    focusHandoverSoon();
    const took = vi.fn();
    render(<List took={took} />);
    await act(async () => {
      vi.runAllTimers();
    });
    expect(took).toHaveBeenCalled();
  });
});

describe("F16 an out-of-room account's banner", () => {
  const session = (overrides: Partial<InstanceInfo>) => ({ id: "s-1", account_id: "f", status: "running", started_at_ms: 1, last_activity_ms: 5, ...overrides }) as unknown as InstanceInfo;

  it("continues its open session in another app (the walk's case), the newest one when several", () => {
    expect(openOnAccount("f", [], [conversation()])).toEqual({ conversation: conversation() });
    const newer = conversation({ id: "c-2", last_activity_ms: 50 });
    expect(openOnAccount("f", [session({})], [conversation(), newer])).toEqual({ conversation: newer });
    expect(openOnAccount("f", [session({ last_activity_ms: 99 })], [newer])).toMatchObject({ session: { id: "s-1" } });
  });

  it("finds nothing (and the button says Open its account) when no session of it is open", () => {
    expect(openOnAccount("f", [session({ status: "finished" })], [conversation({ live: false })])).toBeNull();
    // AI Juggler's own copy of a conversation is its session, not counted twice.
    expect(openOnAccount("f", [], [conversation({ session_id: "s-9" })])).toBeNull();
  });
});
