/**
 * 0.5.1 F98 (F-PL1-15 a): Quit…, a running desktop App closed from AI
 * Juggler and left closed. It sits beside Restart… wherever Restart… is,
 * asks in the same confirm (in Quit's words), and Keep alive waits for
 * the App's next Open.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RestartAppDialog, RestartAppHost, afterWords, askQuit } from "./RestartApp";
import { RailIdleApp } from "./RailIdleApp";
import { headerActions } from "./place-actions";
import { resetToasts, toastEntries } from "./Toast";
import { AppsSection } from "../screens/accounts/AppsSection";
import { DesktopAppsPane } from "../screens/settings/DesktopAppsPane";
import { ConversationsProvider } from "../lib/conversations";
import type { AppModel } from "../lib/apps";
import type { IdleApp } from "../lib/rail-apps";
import type { RailGroup } from "../lib/rail-places";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";

const PLACE: KnownPlaceInfo = {
  id: "app:claude-app:/Users/someone/ClaudeProfiles/work", app_id: "claude-app", app: "Claude app", instance: "work", profile: "work", label: "Claude app · work",
  app_pid: 4242, lost: false, keep_alive: true, paused: null, account_id: null, seen_ms: 0, cannot_start: null, recent: [], found: false,
};

function seed(places: KnownPlaceInfo[]): void {
  Object.defineProperty(window, "__AIJ_PLACES__", { value: { places }, configurable: true, writable: true });
}

afterEach(() => {
  cleanup();
  resetToasts();
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
});

describe("F98 Quit…'s confirm", () => {
  it("says what stops and that it stays closed, Keep alive too when it's on", () => {
    expect(afterWords({ quit: true, keepAlive: true })).toBe("AI Juggler quits it and waits for it to close. It stays closed until you start it again, Keep alive too.");
    expect(afterWords({ quit: true })).toBe("AI Juggler quits it and waits for it to close. It stays closed until you start it again.");
    // Restart's words are unchanged.
    expect(afterWords({})).toBe("AI Juggler quits it, waits for it to close, and starts it again on the same profile.");
    render(<RestartAppDialog ask={{ placeId: PLACE.id, name: "Claude app · work", open: 1, working: 1, quit: true, keepAlive: true }} onClose={() => undefined} />);
    const dialog = screen.getByRole("alertdialog", { name: "Quit Claude app · work?" });
    expect(within(dialog).getByTestId("restart-body").textContent).toBe(
      "1 session is working in it. Its turn stops now; its conversation is kept and can be resumed. AI Juggler quits it and waits for it to close. It stays closed until you start it again, Keep alive too.",
    );
    expect(within(dialog).getByRole("button", { name: "Quit" })).toBeTruthy();
    expect(document.activeElement?.textContent).toBe("Cancel");
  });

  it("Quit quits it, says so, and Keep alive waits for its next Open", async () => {
    const place = { ...PLACE };
    seed([place]);
    render(<RestartAppHost />);
    askQuit({ placeId: PLACE.id, name: "Claude app · work", keepAlive: true });
    fireEvent.click(await screen.findByRole("button", { name: "Quit" }));
    await waitFor(() => expect(toastEntries().some((toast) => toast.options.message === "Quit Claude app · work")).toBe(true));
    // The preview's quit, as the backend's: closed, and Keep alive paused.
    expect(place.app_pid).toBeNull();
    expect(place.paused).toBe("you quit it in AI Juggler");
  });
});

describe("F98 Quit… is beside Restart… wherever Restart… is", () => {
  it("the rail's App header menu, while it runs", () => {
    const group: RailGroup = { key: "g", label: "Claude app · work", place: null, accountId: null, poolId: null, items: [] };
    const running = headerActions(PLACE, group);
    expect(running.onRestart).toBeTypeOf("function");
    expect(running.onQuit).toBeTypeOf("function");
    const closed = headerActions({ ...PLACE, app_pid: null }, group);
    expect(closed.onRestart).toBeUndefined();
    expect(closed.onQuit).toBeUndefined();
  });

  it("an inactive App's rail menu", () => {
    const idle: IdleApp = {
      key: "app|claude-app|work", kind: "desktop", folder: null, name: "Claude app · work", app: "Claude app", profile: "work", appId: "claude-app",
      lastMs: 0, appPid: 4242, placeId: PLACE.id, cannotStart: null, accountId: null, tool: null, sessions: [],
    };
    render(<RailIdleApp app={idle} now={0} open={false} onToggle={() => undefined} />);
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"));
    expect(screen.getByTestId("rail-idle-menu-restart").textContent).toContain("Restart…");
    expect(screen.getByTestId("rail-idle-menu-quit").textContent).toContain("Quit…");
  });

  it("Accounts › Apps' ⋯, only while it runs", () => {
    const app = (place: KnownPlaceInfo): AppModel => ({
      id: place.id, kind: "desktop", tool: "claude-code", appId: "claude-app", app: "Claude app", profile: "work", name: "Claude app · work", defaultName: "Claude app · work", named: false, railKey: place.id, folder: null, accountId: null, place, slot: 0,
    });
    render(<AppsSection apps={[app(PLACE)]} subscriptions={[]} sessions={[]} conversations={[]} now={0} />);
    fireEvent.click(screen.getByTestId("accounts-app-menu"));
    expect(screen.getByTestId("accounts-app-quit").textContent).toBe("Quit…");
    cleanup();
    render(<AppsSection apps={[app({ ...PLACE, app_pid: null })]} subscriptions={[]} sessions={[]} conversations={[]} now={0} />);
    fireEvent.click(screen.getByTestId("accounts-app-menu"));
    expect(screen.queryByTestId("accounts-app-quit")).toBeNull();
  });

  it("Settings › Desktop apps' ⋯", async () => {
    seed([{ ...PLACE }]);
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    fireEvent.click(await screen.findByTestId("desktop-place-more"));
    expect(screen.getByTestId("desktop-place-restart").textContent).toBe("Restart…");
    expect(screen.getByTestId("desktop-place-quit").textContent).toBe("Quit…");
  });
});
