/** D-64 (0.3.4): an app with nothing open, in the rail's Inactive apps:
 * the maintainer's Claude app that quit overnight can be started from the
 * rail, a CLI app gets New session there, and its last ones unfold. */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as places from "../lib/ipc/places";
import { RailIdleApp, idleWords } from "./RailIdleApp";
import type { IdleApp } from "../lib/rail-apps";
import { OPEN_APP } from "../lib/apps";
import type { RailItem } from "../lib/rail-items";

const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const app = (over: Partial<IdleApp> = {}): IdleApp => ({
  key: "app|claude-app|Work",
  kind: "desktop",
  name: "Claude app · Work",
  app: "Claude app",
  profile: "Work",
  appId: "claude-app",
  lastMs: NOW - 3 * 3_600_000,
  appPid: null,
  placeId: "app:claude-app:/p/Work",
  cannotStart: null,
  accountId: null,
  tool: null,
  sessions: [],
  ...over,
});

const sessions = (count: number) => Array.from({ length: count }, (_, n) => ({ key: `c:a/${n}` }) as RailItem);

describe("D-64 an inactive app in the rail", () => {
  it("a desktop app that quit: Start starts it (its menu too), and says when it was last used", () => {
    const open = vi.spyOn(places, "placesOpen").mockResolvedValue({ outcome: "Started" } as never);
    render(<RailIdleApp app={app()} open={false} onToggle={vi.fn()} now={NOW} />);
    // Under "Inactive apps", "Not running" went without saying (design
    // review #48).
    expect(screen.getByTestId("rail-idle-when").textContent).toBe("3 h ago");
    const action = screen.getByTestId("rail-idle-action");
    expect(action.getAttribute("aria-label")).toBe("Start: Claude app · Work");
    fireEvent.click(action);
    expect(open).toHaveBeenCalledWith("app:claude-app:/p/Work");
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"), { clientX: 10, clientY: 10 });
    fireEvent.click(screen.getByTestId("rail-idle-menu-act"));
    expect(open).toHaveBeenCalledTimes(2);
  });

  it("one that can't be started from here says why, and isn't clickable", () => {
    render(<RailIdleApp app={app({ cannotStart: "Its profile folder is gone" })} open={false} onToggle={vi.fn()} now={NOW} />);
    const action = screen.getByTestId("rail-idle-action") as HTMLButtonElement;
    expect(action.disabled).toBe(true);
    expect(action.title).toBe("Its profile folder is gone");
  });

  it("running with nothing open: Bring forward", () => {
    render(<RailIdleApp app={app({ appPid: 42 })} open={false} onToggle={vi.fn()} now={NOW} />);
    expect(screen.getByTestId("rail-idle-action").dataset.action).toBe("Bring forward");
  });

  it("a CLI app: New session on its folder; its row unfolds its last ones", () => {
    const asked = vi.fn();
    window.addEventListener("aij:new-session", asked);
    const onToggle = vi.fn();
    const cli = app({ key: "cli|claude-code|a2", kind: "cli", name: "Claude Code · Personal", app: "Claude Code", profile: "Personal", placeId: null, accountId: "a2", tool: "claude-code", sessions: sessions(5) });
    render(<RailIdleApp app={cli} open={false} onToggle={onToggle} now={NOW} />);
    expect(screen.getByTestId("rail-idle-when").textContent).toBe("3 h ago · 5 sessions");
    fireEvent.click(screen.getByTestId("rail-idle-action"));
    expect((asked.mock.calls[0]![0] as CustomEvent).detail).toEqual({ accountId: "a2" });
    window.removeEventListener("aij:new-session", asked);
    fireEvent.click(screen.getByTestId("rail-fold"));
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"), { clientX: 10, clientY: 10 });
    expect(screen.getByTestId("rail-idle-menu-fold").textContent).toBe("Show its last 3");
    fireEvent.click(screen.getByTestId("rail-idle-menu-fold"));
    expect(onToggle).toHaveBeenCalledTimes(2);
  });

  it("D-62: its name opens its App page (its menu too), a desktop one's and a CLI one's", () => {
    const opened = vi.fn();
    const listen = (event: Event) => opened((event as CustomEvent<{ id: string }>).detail.id);
    window.addEventListener(OPEN_APP, listen);
    const desk = render(<RailIdleApp app={app()} open={false} onToggle={vi.fn()} now={NOW} />);
    const name = screen.getByTitle("Open Claude app · Work's page");
    fireEvent.click(name);
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"), { clientX: 10, clientY: 10 });
    fireEvent.click(screen.getByTestId("rail-idle-menu-page"));
    desk.unmount();
    render(<RailIdleApp app={app({ kind: "cli", placeId: null, accountId: "a2", name: "Claude Code · Personal" })} open={false} onToggle={vi.fn()} now={NOW} />);
    fireEvent.click(screen.getByTitle("Open Claude Code · Personal's page"));
    window.removeEventListener(OPEN_APP, listen);
    expect(opened.mock.calls.map((call) => call[0])).toEqual(["app:claude-app:/p/Work", "app:claude-app:/p/Work", "cli:a2"]);
  });

  it("Keep alive from its menu", () => {
    const toggle = vi.fn();
    render(<RailIdleApp app={app()} open={false} onToggle={vi.fn()} now={NOW} keepAlive={{ on: false, toggle }} />);
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"), { clientX: 10, clientY: 10 });
    fireEvent.click(screen.getByTestId("rail-idle-menu-keep"));
    expect(toggle).toHaveBeenCalledTimes(1);
  });

  it("words: never seen, only whether it runs", () => {
    expect(idleWords(app({ lastMs: 0 }), NOW)).toBe("");
  });
});

describe("0.4.1 a running app's menu in the rail", () => {
  it("Restart… asks first (the confirm), only for a running desktop app", async () => {
    const restart = await import("./RestartApp");
    const ask = vi.spyOn(restart, "askRestart").mockImplementation(() => undefined);
    render(<RailIdleApp app={app({ appPid: 4100 })} open={false} onToggle={vi.fn()} now={NOW} />);
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"), { clientX: 10, clientY: 10 });
    fireEvent.click(screen.getByTestId("rail-idle-menu-restart"));
    expect(ask).toHaveBeenCalledWith({ placeId: "app:claude-app:/p/Work", name: "Claude app · Work", open: 0, working: 0 });
    cleanup();
    render(<RailIdleApp app={app()} open={false} onToggle={vi.fn()} now={NOW} />);
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"), { clientX: 10, clientY: 10 });
    expect(screen.queryByTestId("rail-idle-menu-restart")).toBeNull();
  });
});
