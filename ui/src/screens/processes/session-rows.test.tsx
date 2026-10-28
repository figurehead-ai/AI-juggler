/**
 * The 0.2 walk (F5): a desktop app's processes are listed once, as the
 * app's, shared by its sessions; an untitled session is named by its
 * folder and when it started, never "Untitled session".
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { NameCell, sessionTitle } from "./cells";
import type { ProcessGraphRowInfo } from "../../lib/ipc/processes";

afterEach(cleanup);

const NOW = Date.UTC(2026, 9, 28, 15, 0);

function row(session: Partial<NonNullable<ProcessGraphRowInfo["session"]>>, processes = 3): ProcessGraphRowInfo {
  return {
    key: "s:theirs:tool:x",
    kind: "session",
    depth: 1,
    parent_key: "a:desk",
    has_children: processes > 0,
    state: "working",
    origin: "theirs",
    owner_words: "In a session Desk App runs",
    cpu_pct: 223,
    rss_bytes: 800_000_000,
    ports: [],
    app: null,
    process: null,
    rollup: { processes, rss_bytes: 800_000_000, cpu_pct: 223, ports: [], attention: 0, to_clean: 0, worst: "working" },
    session_key: "s:theirs:tool:x",
    session: {
      session_id: "x",
      origin: "theirs",
      tool: "tool",
      account_id: null,
      project_id: null,
      title: "",
      folder_name: null,
      state: "working",
      alive: true,
      root_pid: 900,
      folded: true,
      shared_by: 0,
      started_ms: null,
      ...session,
    },
  };
}

describe("Processes' session rows", () => {
  it("names the app's shared processes as the app's, once", () => {
    const shared = row({ shared_by: 5, session_id: "" });
    expect(sessionTitle(shared, NOW)).toBe("The app itself");
    const view = render(<NameCell row={shared} open={false} onToggle={() => {}} />);
    expect(view.container.textContent).toContain("Shared by its 5 sessions · 3 processes");
  });

  it("names an untitled session by its folder and when it started", () => {
    const started = NOW - 2 * 3_600_000;
    const title = sessionTitle(row({ folder_name: "shop", started_ms: started }), NOW);
    expect(title).toMatch(/^shop · started /);
    expect(sessionTitle(row({ started_ms: started }), NOW)).toMatch(/^Session started /);
    expect(sessionTitle(row({ folder_name: "shop" }), NOW)).toBe("shop · no title yet");
    expect(sessionTitle(row({}), NOW)).toBe("A session with no title yet");
    expect(sessionTitle(row({ title: "Fix the login test" }), NOW)).toBe("Fix the login test");
    for (const name of [title, sessionTitle(row({}), NOW)]) expect(name).not.toContain("Untitled session");
  });

  it("a session with nothing of its own running says so", () => {
    const view = render(<NameCell row={row({ title: "Blog images" }, 0)} open={false} onToggle={() => {}} />);
    expect(view.container.textContent).toContain("nothing of its own running");
  });
});
