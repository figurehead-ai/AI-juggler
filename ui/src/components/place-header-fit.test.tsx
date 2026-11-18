/**
 * D-100 (0.4.4) with D-118's mock (round 3): an Open now header names its
 * Account, then its app, always ("Work Max Claude app", "Work Max Claude
 * Code CLI"): two Apps on one Account read "Work Max" twice when the app
 * word gave way, told apart by their icons alone. Its other words fit
 * whole or not at all after them: an App that runs with nothing open
 * says "nothing open" when there is room (its spoken name always does).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PlaceHeader } from "./PlaceHeader";

/** Each part's box as measured in the rail at its default 264 px (the
 * design preview, a profile named "Research"): the words' box 91 px after
 * the name and its app word, "nothing open" 74 px. */
const RECTS: Record<string, { left: number; width: number }> = {
  "place-name-line": { left: 100, width: 91 },
  "place-app-word": { left: 100, width: 64 },
  "place-nothing-open": { left: 167, width: 74 },
};

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const rect = RECTS[this.dataset.testid ?? ""] ?? { left: 0, width: 0 };
    return DOMRect.fromRect({ x: rect.left, y: 0, width: rect.width, height: 18 });
  });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const place = { key: "app|claude-app|Research", app: "Claude app", appId: "claude-app", kind: "desktop" as const, profile: "Research", appPid: 7, host: null };
const long = { ...place, key: "app|claude-app|acme-research", profile: "acme-research" };
const shown = (id: string) => {
  const el = screen.queryByTestId(id);
  return el !== null && el.style.display !== "none" && el.style.visibility !== "hidden";
};

describe("D-100 / D-118 an Open now header: its Account, then its app, always", () => {
  it("names the Account, then the app, in that order, the app word never hidden", () => {
    render(<PlaceHeader place={place} label="Claude app" count={2} needs={1} folded={null} onToggle={() => {}} />);
    expect(shown("place-app-word")).toBe(true);
    const words = [...screen.getByTestId("place-page").querySelectorAll("[data-testid='place-profile'], [data-testid='place-app-word']")].map((el) => el.textContent);
    expect(words).toEqual(["Research", "Claude app"]);
    // Cut at its end only when nothing else is left, its whole name on hover.
    expect(screen.getByTestId("place-app-word").className).toContain("truncate");
    expect(screen.getByTestId("place-app-word").getAttribute("title")).toBe("Claude app · Research");
  });

  it("an App with no profile (ChatGPT) is its name alone", () => {
    render(<PlaceHeader place={{ ...place, key: "app|codex-app|", app: "ChatGPT", appId: "codex-app", profile: null }} label="ChatGPT" count={1} needs={0} folded={null} onToggle={() => {}} />);
    expect(screen.queryByTestId("place-app-word")).toBeNull();
    expect(screen.getByTestId("place-page").textContent).toContain("ChatGPT");
  });

  it("at 264 px with nothing open: the app word, then \"nothing open\" when it fits whole", () => {
    render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} foldable={false} />);
    expect(shown("place-app-word")).toBe(true);
    expect(shown("place-nothing-open")).toBe(true);
    expect(screen.getByTestId("place-profile").textContent).toBe("Research");
  });

  it("a longer name (68 px left): \"nothing open\" gives way, never the app word; its name says it", () => {
    RECTS["place-name-line"] = { left: 100, width: 68 };
    try {
      render(<PlaceHeader place={long} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} foldable={false} />);
      expect(shown("place-nothing-open")).toBe(false);
      expect(shown("place-app-word")).toBe(true);
      expect(screen.getByTestId("place-profile").textContent).toBe("acme-research");
      expect(screen.getByTestId("place-page").querySelector(".sr-only")?.textContent).toContain("nothing open");
    } finally {
      RECTS["place-name-line"] = { left: 100, width: 91 };
    }
  });

  it("a render with the same words changes nothing in them (AJ-E805)", async () => {
    // Every render showed each word, measured it and hid it again: an App's
    // header renders with every change, about 400 writes a minute with 10
    // sessions at work, and the header was drawn again for each.
    const { rerender } = render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} foldable={false} />);
    const line = screen.getByTestId("place-name-line");
    const writes: MutationRecord[] = [];
    const watch = new MutationObserver((records) => writes.push(...records));
    watch.observe(line, { subtree: true, attributes: true });
    // Renders whose words are the same (a new handler, a new level): its
    // needs' mark is one of its words now (D-118), so it isn't varied.
    for (const level of [2, 3, 0] as const) rerender(<PlaceHeader place={place} label="Claude app" count={0} needs={0} level={level} folded={null} onToggle={() => {}} foldable={false} />);
    await Promise.resolve();
    watch.disconnect();
    expect(writes).toEqual([]);
    expect(shown("place-nothing-open")).toBe(true);
  });

  it("its needs' mark comes before its count, whole or not at all, after the names", () => {
    render(<PlaceHeader place={place} label="Claude app" count={2} needs={1} level={2} folded={null} onToggle={() => {}} />);
    const line = screen.getByTestId("place-name-line");
    expect(line.querySelector("[data-testid='place-needs']")).not.toBeNull();
    expect(line.lastElementChild?.textContent).toBe("2");
  });

  it("with something open: no \"nothing open\"", () => {
    render(<PlaceHeader place={place} label="Claude app" count={2} needs={0} folded={null} onToggle={() => {}} />);
    expect(shown("place-app-word")).toBe(true);
    expect(screen.queryByTestId("place-nothing-open")).toBeNull();
  });
});
