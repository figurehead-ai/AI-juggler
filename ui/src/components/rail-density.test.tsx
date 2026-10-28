/**
 * AJ-E403 and the 2026-10-27 audit: Settings › Appearance › Density works
 * in the default rail (the place lens drew 44 px rows whatever was
 * chosen), and ⌘B folds and opens the rail without resetting Compact to
 * Comfortable.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { placesConversations } from "../preview/places";
import { readDensity, storeDensity, toggledDensity } from "../lib/density";
import { LINE_HEIGHT, LINE_HEIGHT_COMPACT } from "../lib/rail-lines";

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
type Seams = { __AIJ_FIXTURE__?: unknown };

beforeEach(() => {
  resetRailViewCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  delete (window as unknown as Seams).__AIJ_FIXTURE__;
  resetRailViewCache();
});

function board(density: "comfortable" | "compact") {
  (window as unknown as Seams).__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  return render(
    <ConversationsProvider>
      <InstanceBoard sessions={[]} loginTerminals={[]} selectedId={null} onSelect={() => {}} density={density} freshness={{}} now={NOW} projects={[]} snooze={{}} onSnooze={() => {}} />
    </ConversationsProvider>,
  );
}

describe("density in the rail by place", () => {
  it("Compact draws one-line rows, Comfortable two", async () => {
    const roomy = board("comfortable");
    const roomyBoard = await roomy.findByTestId("place-board");
    expect(roomyBoard.getAttribute("data-density")).toBe("comfortable");
    const roomyRow = roomyBoard.querySelector<HTMLElement>("[data-conversation-id='outside-1']")!.parentElement!;
    // Two lines, 44 px.
    expect(roomyRow.innerHTML).toContain("h-[44px]");
    cleanup();

    const tight = board("compact");
    const tightBoard = await tight.findByTestId("place-board");
    expect(tightBoard.getAttribute("data-density")).toBe("compact");
    const tightRow = tightBoard.querySelector<HTMLElement>("[data-conversation-id='outside-1']")!.parentElement!;
    // One line, 30 px: the title and the state's word, the rest in its name.
    expect(tightRow.innerHTML).toContain("h-[30px]");
    expect(tightRow.innerHTML).not.toContain("h-[44px]");
    expect(tightRow.querySelector("[data-testid='state-word']")?.textContent).toBe("Working");
    expect(tightRow.querySelector("button")!.textContent).toContain("Refactor the settings screen");
  });

  it("a Compact row is shorter, so the long list's window follows it", () => {
    expect(LINE_HEIGHT_COMPACT.row).toBeLessThan(LINE_HEIGHT.row);
    expect(LINE_HEIGHT_COMPACT.need).toBeLessThan(LINE_HEIGHT.need);
    expect(LINE_HEIGHT_COMPACT.header).toBe(LINE_HEIGHT.header);
  });
});

describe("⌘B never resets Compact", () => {
  it("folds to the icons and opens to the density it had", () => {
    storeDensity("compact");
    const folded = toggledDensity(readDensity());
    expect(folded).toBe("minimal");
    storeDensity(folded);
    expect(toggledDensity(readDensity())).toBe("compact");
  });

  it("a Compact chosen before this release comes back too", () => {
    localStorage.setItem("aij.density", "compact");
    const folded = toggledDensity(readDensity());
    storeDensity(folded);
    expect(toggledDensity(readDensity())).toBe("compact");
  });

  it("opens to Comfortable when that was the last", () => {
    storeDensity("comfortable");
    storeDensity("minimal");
    expect(toggledDensity(readDensity())).toBe("comfortable");
  });
});
