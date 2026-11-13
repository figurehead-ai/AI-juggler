/**
 * AJ-E3208 and D-71: the Accounts screen's Apps beside its Accounts, each
 * with its Account, what's open in it and its one action; Restart… on a
 * running one asks first, naming what it stops.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AppsSection, appsInOrder, stateWords } from "./AppsSection";
import { RestartAppDialog, RestartAppHost, askRestart, restartWords } from "../../components/RestartApp";
import { newestCwd, type AccountApp } from "../../lib/account-items";
import type { AppModel } from "../../lib/apps";
import type { KnownPlaceInfo } from "../../types/generated/KnownPlaceInfo";

afterEach(() => cleanup());

const app = (id: string, over: Partial<AppModel> = {}): AppModel =>
  ({ id, kind: "desktop", tool: "claude-code", appId: "claude-app", app: "Claude app", profile: id, name: `Claude app · ${id}`, defaultName: `Claude app · ${id}`, named: false, railKey: id, folder: null, accountId: null, place: null, slot: 0, ...over }) as AppModel;
const row = (model: AppModel, open = 0, sessions = 0, lastMs = 0): AccountApp => ({ app: model, open, working: 0, sessions, lastMs });

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
    expect(restartWords(0, 0)).toBe("Nothing is open in it now.");
    expect(restartWords(1, 1)).toMatch(/^1 session is working in it\. Its turn stops now/);
    expect(restartWords(2, 2)).toMatch(/^2 sessions are working in it\. Their turns stop now/);
    // The go sitting: one open and idle is "open", never "working".
    expect(restartWords(1, 0)).toMatch(/^1 session is open in it, none working\. It closes; its conversation is kept/);
    expect(restartWords(3, 1)).toMatch(/^1 session is working in it \(2 more open\)\./);
    // A door that doesn't hold its sessions says so plainly.
    expect(restartWords()).toBe("Anything open in it closes; its conversations are kept and can be resumed.");
  });

  it("asks first, with Cancel focused, and Escape closes it", () => {
    let closed = false;
    render(<RestartAppDialog ask={{ placeId: "app:claude-app:/p/Work", name: "Claude app · Work", open: 2, working: 2 }} onClose={() => (closed = true)} />);
    const dialog = screen.getByRole("alertdialog", { name: "Restart Claude app · Work?" });
    expect(within(dialog).getByTestId("restart-body").textContent).toContain("2 sessions are working in it");
    expect(document.activeElement?.textContent).toBe("Cancel");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(closed).toBe(true);
  });

  it("any entry point asks through the one host", async () => {
    render(<RestartAppHost />);
    expect(screen.queryByTestId("restart-dialog")).toBeNull();
    askRestart({ placeId: "app:claude-app:/p/Work", name: "Claude app · Work", open: 0, working: 0 });
    await waitFor(() => expect(screen.getByTestId("restart-dialog")).toBeTruthy());
    fireEvent.click(screen.getByText("Cancel"));
    await waitFor(() => expect(screen.queryByTestId("restart-dialog")).toBeNull());
  });
});

describe("0.3.8 an app that lost its window (AJ-E3208 R4)", () => {
  const place: KnownPlaceInfo = {
    id: "place-lost", app_id: "claude-app", app: "Claude app", instance: null, profile: "lost", label: "Claude app · lost",
    app_pid: 4242, lost: true, keep_alive: false, paused: null, account_id: null, seen_ms: 0, cannot_start: null, recent: [], found: false,
  };
  const lost = () => app("lost", { place });

  it("says so on its row, before what's open in it", () => {
    expect(stateWords(row(lost(), 2), 0)).toBe("Lost its window when the system restarted the display");
  });

  it("keeps Restart… in its ⋯, as a running app's", () => {
    render(<AppsSection apps={[lost()]} subscriptions={[]} sessions={[]} conversations={[]} now={0} />);
    expect(screen.getByTestId("accounts-app").textContent).toContain("Lost its window");
    fireEvent.click(screen.getByTestId("accounts-app-menu"));
    expect(screen.getByTestId("accounts-app-restart").textContent).toContain("Restart…");
  });
});

describe("D-100 a CLI App says what it is", () => {
  const HOUR = 3_600_000;
  it("CLI · last used <when> in its newest session's folder, home-relative", () => {
    const cli = app("cli:a1", { kind: "cli", app: "Claude Code CLI", name: "Claude Code CLI · acme-south" });
    expect(stateWords({ ...row(cli, 0, 3, 8 * HOUR), lastCwd: "/Users/someone/lab/aj-processes" }, 10 * HOUR)).toBe("Last used 2 h ago in ~/lab/aj-processes");
    // A desktop App doesn't name a folder.
    expect(stateWords({ ...row(app("d"), 0, 3, 8 * HOUR), lastCwd: "/Users/someone/x" }, 10 * HOUR)).toBe("Last used 2 h ago");
  });

  it("D-103.5: a closed desktop App says so, and when it was last open", () => {
    const place: KnownPlaceInfo = {
      id: "place-d", app_id: "claude-app", app: "Claude app", instance: null, profile: "d", label: "Claude app · d",
      app_pid: null, lost: false, keep_alive: false, paused: null, account_id: null, seen_ms: 10 * HOUR - 12 * 60_000, cannot_start: null, recent: [], found: false,
    };
    expect(stateWords(row(app("d", { place }), 0, 3, 8 * HOUR), 10 * HOUR)).toBe("Closed · last open 12 min ago");
  });

  it("each row shows its kind, by icon and word, before its line", () => {
    const cli = app("cli:a1", { kind: "cli", app: "Claude Code CLI", name: "Claude Code CLI · work", profile: "work" });
    render(<AppsSection apps={[app("desk", { profile: "work" }), cli]} subscriptions={[]} sessions={[]} conversations={[]} now={0} />);
    const kinds = screen.getAllByTestId("app-kind").map((mark) => [mark.getAttribute("title"), mark.querySelector("[data-testid='app-kind-word']")?.textContent]);
    expect(kinds).toEqual(expect.arrayContaining([["Desktop app", "Desktop"], ["CLI app", "CLI"]]));
  });
});

describe("D-100 newestCwd", () => {
  it("is the newest item's folder, a session's before a conversation's", () => {
    const item = (lastMs: number, cwd: string | null) => ({ lastMs, session: null, conversation: { cwd } });
    expect(newestCwd([item(1, "/a"), item(3, "/b"), item(5, null)])).toBe("/b");
    expect(newestCwd([])).toBeNull();
  });
});
