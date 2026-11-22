/**
 * AJ-E1610 — a desktop App's page says what its app allows from here:
 * "Open here" by the app's own documented links, sent only while it runs,
 * and one plain line for each part with no sound way in. An app that
 * declares nothing shows nothing.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppModel } from "../../lib/apps";
import type { KnownPlaceInfo } from "../../types/generated/KnownPlaceInfo";
import type { AppReachInfo } from "../../types/generated/AppReachInfo";

const sent: [string, string, string | null][] = [];
vi.mock("../../lib/ipc/places", () => ({
  placesOpenRoute: (id: string, action: string, folder: string | null) => {
    sent.push([id, action, folder]);
    return Promise.resolve({ outcome: "Opened a new chat in Claude app · Work", reached: "thread" });
  },
}));

const { AppReach } = await import("./AppReach");

afterEach(() => {
  cleanup();
  sent.length = 0;
});

const CLAUDE: AppReachInfo = {
  seen: "installed",
  open_here: ["new_chat", "new_in_folder"],
  routes: [
    { action: "new_chat", status: "documented", usable: true },
    { action: "continue_session", status: "unverified", usable: false },
  ],
  track_only: [
    { part: "reply", why: "The Claude app has no way for another app to answer its sessions, so open the session and answer it there." },
    { part: "sessions", why: "Chat and Cowork conversations live on claude.ai, not on this Mac, so only Code sessions show here." },
  ],
};

const CURSOR: AppReachInfo = {
  seen: "source",
  open_here: [],
  routes: [],
  track_only: [
    { part: "account", why: "Cursor has no documented way to open it on one account, so it opens as you last used it." },
    { part: "session_link", why: "Cursor has no link that opens one agent chat, so it comes forward and you pick the chat." },
  ],
};

function app(reach: AppReachInfo | null, pid: number | null): AppModel {
  const place: KnownPlaceInfo = {
    id: "app:claude-app:/Users/you/Profiles/Work",
    app_id: "claude-app",
    app: "Claude",
    instance: "Work",
    profile: "Work",
    label: "Claude app · Work",
    app_pid: pid,
    lost: false,
    keep_alive: false,
    paused: null,
    account_id: null,
    seen_ms: 0,
    cannot_start: null,
    recent: [],
    found: false,
    reach,
  };
  return { id: place.id, kind: "desktop", tool: "claude-code", appId: "claude-app", app: "Claude app", profile: "Work", name: "Claude app · Work", defaultName: "Claude app · Work", named: false, railKey: place.id, folder: null, accountId: null, place, slot: 0 };
}

describe("AJ-E1610 what works from here", () => {
  it("opens a new chat and a new session in the project by the app's own links, to this profile", async () => {
    render(<AppReach app={app(CLAUDE, 4242)} folder={{ name: "my app", root: "/Users/you/Dev/my app" }} />);
    fireEvent.click(screen.getByTestId("app-reach-new-chat"));
    fireEvent.click(screen.getByTestId("app-reach-new-in-folder"));
    await waitFor(() => expect(sent).toHaveLength(2));
    expect(sent).toEqual([
      ["app:claude-app:/Users/you/Profiles/Work", "new_chat", null],
      ["app:claude-app:/Users/you/Profiles/Work", "new_in_folder", "/Users/you/Dev/my app"],
    ]);
    expect(screen.getByTestId("app-reach-new-in-folder").textContent).toBe("New session in my app");
  });

  it("says each part it can't reach in one line, and sends nothing while the app is closed", () => {
    render(<AppReach app={app(CLAUDE, null)} folder={null} />);
    const chat = screen.getByTestId("app-reach-new-chat");
    expect(chat).toHaveProperty("disabled", true);
    expect(screen.getByTestId("app-reach-open").textContent).toContain("Start it first.");
    expect(screen.queryByTestId("app-reach-new-in-folder")).toBeNull();
    expect(screen.getByTestId("app-reach-why-reply").textContent).toBe(CLAUDE.track_only[0]!.why);
    fireEvent.click(chat);
    expect(sent).toHaveLength(0);
  });

  it("an app with no link of its own only says why; one that declares nothing shows nothing", () => {
    render(<AppReach app={app(CURSOR, 7)} folder={{ name: "p", root: "/p" }} />);
    expect(screen.queryByTestId("app-reach-open")).toBeNull();
    expect(screen.getAllByRole("listitem").map((row) => row.textContent)).toEqual(CURSOR.track_only.map((track) => track.why));
    cleanup();
    const { container } = render(<AppReach app={app(null, 7)} folder={null} />);
    expect(container.textContent).toBe("");
  });
});
