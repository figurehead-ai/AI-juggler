/**
 * 0.4.4 (D-100): a session header's chips give way in steps, the App's
 * name last. P3 put the day in an ended session's state chip ("Stopped
 * with an error yesterday 10:26 PM") and the App chip beside it was cut
 * to "· W" at 1280 px. Layout is stood in for here: a part drawn is 7 px
 * a character; one left out (`data-gone`) is none.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { pastDay, pastWhen } from "../../lib/time-words";
import { SessionChips } from "./SessionHead";
import type { SessionModel } from "./model";

const NOW = new Date(2026, 9, 28, 14, 0).getTime();
const YESTERDAY = new Date(2026, 9, 27, 22, 26).getTime();
const CHAR = 7;
let room = 0;

/** What a row draws: its words, a stand-in's own (`data-day`), nothing
 * of a part left out. */
function drawn(node: Node): string {
  if (!(node instanceof HTMLElement)) return node.textContent ?? "";
  if (node.hasAttribute("data-gone")) return "";
  if (node.dataset.day !== undefined) return node.dataset.day;
  return Array.from(node.childNodes, drawn).join("");
}

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const width = this.dataset.testid !== "row" ? 0 : this.style.width === "max-content" ? drawn(this).length * CHAR : room;
    return { width, height: 20, x: 0, y: 0, top: 0, left: 0, right: width, bottom: 20, toJSON: () => ({}) };
  });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const ended: SessionModel = {
  live: false,
  state: { words: "Stopped with an error", tone: "danger", since: YESTERDAY, working: false, asks: false },
  places: [],
  tool: "claude-code",
  sessionId: "s1",
  conversationId: null,
  switchSource: "s1",
  viewedMs: null,
  cwd: "/Users/you/Dev/ai-juggler",
  model: null,
  profileId: "claude-work",
  profileLabel: "Work",
};

function header(model: SessionModel, width: number) {
  room = width;
  return render(
    <div data-testid="row">
      <SessionChips model={model} account={undefined} now={NOW} />
    </div>,
  );
}

const gone = (view: ReturnType<typeof render>, id: string) => view.getByTestId(id).hasAttribute("data-gone");

it("gives way in steps: the time to its day, the tool word, the time; the App's name never", () => {
  const words = "Stopped with an error";
  const full = ` ${pastWhen(YESTERDAY, NOW)}`;
  const day = ` ${pastDay(YESTERDAY, NOW)}`;
  expect(day).toBe(" yesterday");
  const tool = "Claude Code CLI · ";
  const name = "Work";
  const steps = [words + full + tool + name, words + day + tool + name, words + day + name, words + name];
  steps.forEach((line, step) => {
    const view = header(ended, line.length * CHAR);
    expect(drawn(view.getByTestId("row")), `step ${step}`).toBe(line);
    // Read whole and on hover, whatever is drawn.
    expect(view.getByTestId("session-state-chip").textContent).toBe(words + full);
    expect(view.getByTestId("session-state-chip").title).toBe(words + full);
    expect(gone(view, "session-ended-at")).toBe(step >= 1);
    expect(view.getByTestId("session-ended-at").className).toContain("data-gone:sr-only");
    expect(gone(view, "session-ended-day")).toBe(step === 0 || step === 3);
    expect(gone(view, "session-app-word-part")).toBe(step >= 2);
    expect(gone(view, "session-app-short")).toBe(false);
    cleanup();
  });
  // Narrower still: nothing more is left out; the name's own truncation is the last.
  const view = header(ended, 10 * CHAR);
  expect(drawn(view.getByTestId("row"))).toBe(words + name);
});

it("a time today is its own short form: it stays until the time goes", () => {
  const today = new Date(2026, 9, 28, 9, 5).getTime();
  const model: SessionModel = { ...ended, state: { ...ended.state, since: today } };
  const line = `Stopped with an error ${pastWhen(today, NOW)}Work`;
  const view = header(model, line.length * CHAR);
  expect(view.queryByTestId("session-ended-day")).toBeNull();
  expect(gone(view, "session-ended-at")).toBe(false);
  expect(gone(view, "session-app-word-part")).toBe(true);
  expect(drawn(view.getByTestId("row"))).toBe(line);
});

it("with room, all of it whole, as before", () => {
  const view = header(ended, 2000);
  expect(drawn(view.getByTestId("row"))).toBe(`Stopped with an error ${pastWhen(YESTERDAY, NOW)}Claude Code CLI · Work`);
  expect(view.getByTestId("session-app-chip").textContent).toBe("Claude Code CLI · Work");
});

// 0.4.4 (D-100): at 1024 px an ended session's four actions left its
// chips 96 px and the App chip read nothing; no step fits that. The room
// the chips sit in hears what they need at their last step, the same at
// every width, and a page header moves its actions down a line for it.
it("tells the room it sits in what it needs with all it can leave out left out, at any width", () => {
  const words = "Stopped with an error";
  const name = "Work";
  for (const width of [2000, 40 * CHAR, 10 * CHAR]) {
    room = width;
    const view = render(
      <div data-give-way-room="" data-testid="room">
        <div data-testid="row">
          <SessionChips model={ended} account={undefined} now={NOW} />
        </div>
      </div>,
    );
    expect(view.getByTestId("room").style.getPropertyValue("--give-way-need"), `at ${width}`).toBe(`${(words + name).length * CHAR}px`);
    // Telling it moved nothing: the row keeps the step its width takes.
    expect(gone(view, "session-app-word-part"), `at ${width}`).toBe(width < (words + ` ${pastDay(YESTERDAY, NOW)}Claude Code CLI · ${name}`).length * CHAR);
    cleanup();
  }
});

