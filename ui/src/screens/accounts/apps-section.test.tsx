/**
 * AJ-E3208 and D-71: the Accounts screen's Apps beside its Accounts, each
 * with its Account, what's open in it and its one action; Restart… on a
 * running one asks first, naming what it stops.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { appsInOrder } from "./AppsSection";
import { RestartAppDialog, RestartAppHost, askRestart, restartWords } from "../../components/RestartApp";
import type { AccountApp } from "../../lib/account-items";
import type { AppModel } from "../../lib/apps";

afterEach(() => cleanup());

const app = (id: string, over: Partial<AppModel> = {}): AppModel =>
  ({ id, kind: "desktop", tool: "claude-code", appId: "claude-app", app: "Claude app", profile: id, name: `Claude app · ${id}`, defaultName: `Claude app · ${id}`, named: false, railKey: id, folder: null, accountId: null, place: null, slot: 0, ...over }) as AppModel;
const row = (model: AppModel, open = 0, sessions = 0, lastMs = 0): AccountApp => ({ app: model, open, sessions, lastMs }) as AccountApp;

describe("AJ-E3208 the Apps, in order", () => {
  it("running first, then the rest by when they were used, then the ones found and never opened", () => {
    const place = (pid: number | null, found = false) => ({ app_pid: pid, found }) as unknown as AppModel["place"];
    const order = appsInOrder([
      row(app("found", { place: place(null, true) })),
      row(app("old", { place: place(null) }), 0, 3, 100),
      row(app("running", { place: place(4242) })),
      row(app("recent", { place: place(null) }), 0, 2, 900),
    ]).map((one) => one.app.id);
    expect(order).toEqual(["running", "recent", "old", "found"]);
  });
});

describe("D-71 Restart, after a confirm", () => {
  it("says what it stops", () => {
    expect(restartWords(0)).toBe("Nothing is working in it now.");
    expect(restartWords(1)).toMatch(/^1 session is working in it\. It stops now/);
    expect(restartWords(2)).toMatch(/^2 sessions are working in it\. They stop now/);
  });

  it("asks first, with Cancel focused, and Escape closes it", () => {
    let closed = false;
    render(<RestartAppDialog ask={{ placeId: "app:claude-app:/p/Work", name: "Claude app · Work", working: 2 }} onClose={() => (closed = true)} />);
    const dialog = screen.getByRole("alertdialog", { name: "Restart Claude app · Work?" });
    expect(within(dialog).getByTestId("restart-body").textContent).toContain("2 sessions are working in it");
    expect(document.activeElement?.textContent).toBe("Cancel");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(closed).toBe(true);
  });

  it("any entry point asks through the one host", async () => {
    render(<RestartAppHost />);
    expect(screen.queryByTestId("restart-dialog")).toBeNull();
    askRestart({ placeId: "app:claude-app:/p/Work", name: "Claude app · Work", working: 0 });
    await waitFor(() => expect(screen.getByTestId("restart-dialog")).toBeTruthy());
    fireEvent.click(screen.getByText("Cancel"));
    await waitFor(() => expect(screen.queryByTestId("restart-dialog")).toBeNull());
  });
});
