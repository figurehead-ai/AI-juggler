/**
 * ADR-0032 Amendment 3 (D-100): an App's kind by icon and word, beside
 * its icon: two Apps on one Account never look like one App twice.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AppKindBadge } from "./AppKindBadge";
import { appKindOfKey, appKindWords } from "../lib/app-kind";

afterEach(() => cleanup());

describe("D-100 an App's kind", () => {
  it("comes from its rail key or its backend id", () => {
    expect(appKindOfKey("cli|claude-code|a1")).toBe("cli");
    expect(appKindOfKey("cli:a1")).toBe("cli");
    expect(appKindOfKey("app|claude-app|/p/Work")).toBe("desktop");
    expect(appKindOfKey("app:codex-app:")).toBe("desktop");
    // AI Juggler's own place, a pool, nothing: none.
    expect(appKindOfKey("own")).toBeNull();
    expect(appKindOfKey("pool:1")).toBeNull();
    expect(appKindOfKey(null)).toBeNull();
  });

  it("is said in the maintainer's words, never 'terminal'", () => {
    expect(appKindWords("desktop")).toEqual({ word: "Desktop", name: "Desktop app" });
    expect(appKindWords("cli")).toEqual({ word: "CLI", name: "CLI app" });
  });

  it("draws a mark named in words for assistive technology, and its word as a chip", () => {
    render(
      <>
        <AppKindBadge kind="cli" />
        <AppKindBadge kind="desktop" variant="chip" />
        <AppKindBadge kind="cli" variant="chip" decorative />
        <AppKindBadge kind={null} />
      </>,
    );
    const [cli, desktop, quiet] = screen.getAllByTestId("app-kind");
    expect(screen.getAllByTestId("app-kind")).toHaveLength(3);
    // The glyph alone: its name for screen readers and its tooltip.
    expect(cli!.textContent).toBe("CLI app");
    expect(cli!.getAttribute("title")).toBe("CLI app");
    // The chip: its word on screen, its name read.
    expect(desktop!.getAttribute("data-kind")).toBe("desktop");
    expect(desktop!.querySelector("[data-testid='app-kind-word']")!.textContent).toBe("Desktop");
    expect(desktop!.querySelector(".sr-only")!.textContent).toBe("Desktop app");
    // Beside words that say it already: hidden from assistive technology.
    expect(quiet!.getAttribute("aria-hidden")).toBe("true");
    expect(quiet!.querySelector(".sr-only")).toBeNull();
  });
});
