/**
 * AJ-E1510 vitest — Settings › Notifications' macOS notifications: five
 * kinds, every one off until chosen, each saved on its own and said in
 * words; the quiet hours and Accounts say they apply to notifications
 * too; and a clicked notification reaches the app (the preview's
 * stand-in event) as its session or This Mac.
 */
import { act, cleanup, fireEvent, render, renderHook, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsScreen } from "./Settings";
import { useOutsideRequests } from "../lib/outside";
import { NOTIFICATION_CLICK_EVENT } from "../lib/ipc/notifications";

const w = () => window as unknown as Record<string, unknown>;

afterEach(cleanup);
beforeEach(() => {
  for (const key of ["__AIJ_NOTIFY__", "__AIJ_NOTIFICATION_PREFS__", "__AIJ_APP_SETTINGS__"]) delete w()[key];
});

const LABELS = [
  "A session waits for you",
  "A session looks stuck",
  "A session hits its usage limit",
  "A session finishes",
  "A session leaves something running",
  "This Mac is getting tight",
];

describe("AJ-E1510 — Settings › Notifications", () => {
  it("offers six kinds, all off until chosen", async () => {
    const { findByTestId } = render(<SettingsScreen focusSection="notifications" />);
    const group = await findByTestId("system-notifications");
    for (const label of LABELS) {
      expect((within(group).getByLabelText(label) as HTMLInputElement).checked).toBe(false);
    }
    expect(group.textContent).toContain("never while AI Juggler is in front");
  });

  it("saves each kind on its own and says what changed", async () => {
    const { findByTestId, getByTestId } = render(<SettingsScreen focusSection="notifications" />);
    const group = await findByTestId("system-notifications");
    await act(async () => {
      fireEvent.click(within(group).getByLabelText("A session finishes"));
    });
    expect(w().__AIJ_NOTIFY__).toEqual({ waiting: false, stuck: false, limit: false, finished: true, this_mac: false, left_behind: false, quota: false, crash: false, reset: false });
    await waitFor(() =>
      expect(getByTestId("settings-notifications").textContent).toContain("You'll get a notification when a session finishes."),
    );
    await act(async () => {
      fireEvent.click(within(group).getByLabelText("A session finishes"));
    });
    expect(w().__AIJ_NOTIFY__).toMatchObject({ finished: false });
    await waitFor(() => expect(getByTestId("settings-notifications").textContent).toContain("No notification when a session finishes."));
  });

  it("says the quiet hours and the Accounts left out apply to notifications too", async () => {
    const { findByTestId } = render(<SettingsScreen focusSection="notifications" />);
    expect((await findByTestId("quiet-hours")).textContent).toContain("The digest and notifications wait until the quiet hours end");
    expect((await findByTestId("digest-accounts")).textContent).toContain("In the digest and notifications");
  });
});

describe("AJ-E1510 — a clicked notification", () => {
  it("opens its session, or This Mac", () => {
    const open = vi.fn();
    const { unmount } = renderHook(() => useOutsideRequests({ hotkey: () => {}, goTo: () => {}, open }));
    const click = (detail: unknown) => window.dispatchEvent(new CustomEvent(NOTIFICATION_CLICK_EVENT, { detail }));
    click({ session_id: "sess-live-1", page: null });
    click({ session_id: null, page: "this_mac" });
    expect(open.mock.calls).toEqual([[{ session_id: "sess-live-1", page: null }], [{ session_id: null, page: "this_mac" }]]);
    unmount();
    click({ session_id: "sess-live-1", page: null });
    expect(open).toHaveBeenCalledTimes(2);
  });
});
