/**
 * 0.5.1 F60 (N8): every menu line carries its icon, as every menu's
 * (`MenuLine`): the feed's ⋯, an inactive App's rail menu, and an open
 * App's header menu, which had lines of words alone.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeFeedFilters } from "./HomeFeedFilters";
import { PlaceHeader } from "./PlaceHeader";
import { RailIdleApp } from "./RailIdleApp";
import type { IdleApp } from "../lib/rail-apps";

const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();

afterEach(cleanup);

/** Each line of the open menu `testId`, with whether it has an icon. */
function lines(testId: string): { words: string; icon: boolean }[] {
  return [...screen.getByTestId(testId).querySelectorAll<HTMLElement>("[role^='menuitem']")].map((line) => ({
    words: line.textContent ?? "",
    icon: line.querySelector("svg[aria-hidden='true']") !== null,
  }));
}

const idle: IdleApp = {
  key: "app|claude-app|Work", kind: "desktop", folder: null, name: "Claude app · Work", app: "Claude app", profile: "Work", appId: "claude-app",
  lastMs: NOW - 3 * 3_600_000, appPid: 4100, placeId: "app:claude-app:/p/Work", cannotStart: null, accountId: null, tool: null,
  sessions: [],
};

describe("menus carry an icon on every line (F60)", () => {
  it("the feed's ⋯: Include my actions (its ✓ while on), Mark all read, Mark all reviewed", () => {
    render(
      <HomeFeedFilters filters={{ category: "finished", include_mine: true, project: null, pool: null, folders: null }} onChange={vi.fn()}
        facets={{ projects: [], accounts: [], subscriptions: [], pools: null }} toReview={3} onReviewAll={vi.fn()} onReadAll={vi.fn()} />,
    );
    fireEvent.click(screen.getByTestId("feed-more"));
    const shown = lines("feed-menu");
    expect(shown.map((line) => line.words)).toEqual([
      "Include my actions✓",
      "Mark all readClears the new dots in the feed",
      "Mark all 3 reviewedMarks these finished sessions as looked at",
    ]);
    expect(shown.every((line) => line.icon)).toBe(true);
    expect(screen.getByTestId("feed-read-all-label").textContent).toBe("Mark all read");
    expect(screen.getByTestId("feed-review-all-label").textContent).toBe("Mark all 3 reviewed");
  });

  it("an inactive App's rail menu: its action, Restart…, Keep alive, Open its page", () => {
    render(<RailIdleApp app={idle} open={false} onToggle={vi.fn()} now={NOW} keepAlive={{ on: false, toggle: vi.fn() }} />);
    fireEvent.contextMenu(screen.getByTestId("rail-idle-app"), { clientX: 10, clientY: 10 });
    const shown = lines("rail-idle-menu");
    expect(shown.length).toBeGreaterThanOrEqual(4);
    expect(shown.filter((line) => !line.icon)).toEqual([]);
  });

  it("an open App's header menu", () => {
    render(<PlaceHeader place={null} label="Work" count={3} needs={0} folded={null} onToggle={vi.fn()} onOpen={vi.fn()} onPage={vi.fn()} onNew={vi.fn()} keepAlive={{ on: true, toggle: vi.fn() }} />);
    fireEvent.contextMenu(screen.getByTestId("place-header"), { clientX: 20, clientY: 30 });
    const shown = lines("place-header-menu");
    expect(shown.length).toBeGreaterThanOrEqual(4);
    expect(shown.filter((line) => !line.icon)).toEqual([]);
  });
});
