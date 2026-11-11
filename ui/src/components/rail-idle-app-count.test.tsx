/** Walk K1 (AJ-E3306, rc24): an inactive App's row in the rail said "67
 * sessions" where its App page and the database said 333: it counted the
 * recent sessions the rail had loaded. Its count is its App page's (the
 * backend's relation counts), and none shows until that's known. */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { IdleApp } from "../lib/rail-apps";
import { sessionItem } from "../lib/rail-items";
import { MOCK_SESSIONS } from "../preview/fixtures";

const { invoke, quiet } = vi.hoisted(() => ({
  invoke: vi.fn(async (_command: string, args: { entities: { kind: string; id: string }[] }) =>
    args.entities.map((entity) => ({ ...entity, sessions: { all: entity.id === "cli:work" ? 334 : 333, open: 0, ended: 333, archived: 0 } })),
  ),
  quiet: () => () => undefined,
}));
vi.mock("../lib/ipc/transport", async (real) => ({ ...(await real<object>()), liveBackend: true, call: (command: string, args: never) => invoke(command, args) }));
vi.mock("../lib/ipc/instances", async (real) => ({ ...(await real<object>()), onSessionsUpdated: quiet }));
vi.mock("../lib/ipc/conversations", async (real) => ({ ...(await real<object>()), onConversationsChanged: quiet }));
vi.mock("../lib/ipc/places", async (real) => ({ ...(await real<object>()), onPlacesChanged: quiet }));
vi.mock("../lib/pools", async (real) => ({ ...(await real<object>()), onPoolsChanged: quiet }));

afterEach(() => {
  cleanup();
  invoke.mockClear();
});

const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();
/** The sessions the rail has loaded for it: a window, not all of them. */
const loaded = (count: number) => Array.from({ length: count }, () => sessionItem(MOCK_SESSIONS[0]!, () => null, false));
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
  sessions: loaded(67),
  ...over,
});

describe("an inactive App's session count in the rail", () => {
  it("is its App's real count, not the 67 the rail loaded", async () => {
    const { RailIdleApp } = await import("./RailIdleApp");
    render(<RailIdleApp app={app()} open={false} onToggle={vi.fn()} now={NOW} />);
    // Not the loaded window while the count is read.
    expect(screen.getByTestId("rail-idle-when").textContent).toBe("3 h ago");
    await waitFor(() => expect(screen.getByTestId("rail-idle-when").textContent).toBe("3 h ago · 333 sessions"));
    expect(invoke).toHaveBeenCalledWith("relations_counts", { entities: [{ kind: "app", id: "app:claude-app:/p/Work" }] });
  });

  it("a CLI App asks for its folder's App", async () => {
    const { RailIdleApp } = await import("./RailIdleApp");
    const { cliAppId } = await import("../lib/app-name");
    render(<RailIdleApp app={app({ kind: "cli", placeId: null, accountId: "work", key: "cli|claude|work", name: "Claude Code · Work" })} open={false} onToggle={vi.fn()} now={NOW} />);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("relations_counts", { entities: [{ kind: "app", id: cliAppId("work") }] }));
    expect(screen.getByTestId("rail-idle-when").textContent).not.toContain("67");
  });
});
