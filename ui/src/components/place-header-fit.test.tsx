/**
 * D-100 (0.4.4): an Open now header's secondary words fit whole or not at
 * all, its name giving way last. An App that runs with nothing open lost
 * "nothing open" at the rail's default 264 px: its app word ("Claude
 * app") came first and took the room, leaving a bare profile name. Now
 * "nothing open" is placed first (the kind mark and the icon already say
 * which app), and the app word shows only in what's left.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PlaceHeader } from "./PlaceHeader";

/** Each part's box as measured in the rail at its default 264 px (the
 * design preview, a profile named "Research"): the words' box 91 px after
 * the name, "Claude app" 64 px, "nothing open" 74 px, 3 px apart. Before,
 * the app word took the box and "nothing open" was hidden. */
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

describe("D-100 an App running with nothing open keeps its words at 264 px", () => {
  it("at 264 px, no room for both: \"nothing open\" shows, the app word gives way", () => {
    render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} foldable={false} />);
    expect(shown("place-nothing-open")).toBe(true);
    expect(shown("place-app-word")).toBe(false);
    // The name stays whole beside them (it gives way last).
    expect(screen.getByTestId("place-profile").textContent).toBe("Research");
  });

  it("at 264 px, a longer name (68 px left, room for the app word alone): neither; the app word never stands in, the name stays whole", () => {
    RECTS["place-name-line"] = { left: 100, width: 68 };
    try {
      render(<PlaceHeader place={long} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} foldable={false} />);
      expect(shown("place-nothing-open")).toBe(false);
      expect(shown("place-app-word")).toBe(false);
      expect(screen.getByTestId("place-profile").textContent).toBe("acme-research");
    } finally {
      RECTS["place-name-line"] = { left: 100, width: 91 };
    }
  });

  it("room for both: both, the app word first", () => {
    RECTS["place-name-line"] = { left: 100, width: 160 };
    try {
      render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} foldable={false} />);
      expect(shown("place-app-word")).toBe(true);
      expect(shown("place-nothing-open")).toBe(true);
      const line = screen.getByTestId("place-name-line");
      expect([...line.children].map((child) => child.getAttribute("data-testid"))).toEqual(["place-app-word", "place-nothing-open"]);
    } finally {
      RECTS["place-name-line"] = { left: 100, width: 91 };
    }
  });

  it("with something open, the app word is all there is: shown while it fits whole", () => {
    render(<PlaceHeader place={place} label="Claude app" count={2} needs={0} folded={null} onToggle={() => {}} />);
    expect(shown("place-app-word")).toBe(true);
    expect(screen.queryByTestId("place-nothing-open")).toBeNull();
  });
});
