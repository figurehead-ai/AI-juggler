/**
 * D-126 — ToolLogo: a desktop app's own installed icon; a CLI its
 * vendor's desktop app icon (the catalog's `host_app`) with a terminal
 * badge; with no icon on this Mac a filled neutral squircle, the tool's
 * mark where its owner allows it, else its letter (OpenCode, Kimi and
 * Grok, whose rules aren't logged yet). No branch on a tool: the catalog
 * and the marks registry say it all.
 *
 * The icons here are made-up test images; no app's real icon is in the
 * repository.
 */
import { act, cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { forgetAppIconsForTests } from "./AppIcon";
import { LogoTile, ToolLogo } from "./ToolLogo";
import { IconInfo } from "./icons-actions";
import { genericTool, setToolsForTests } from "../lib/tools";
import { PREVIEW_TOOLS } from "../preview/tools";

vi.mock("../assets/third-party-marks", async (actual) => {
  const real = await actual<typeof import("../assets/third-party-marks")>();
  // A made-up mark whose owner allows it: a plain square.
  return { MARKS: { ...real.MARKS, allowedtest: { owner: "Test", allowed: true, path: "M4 4h16v16H4z" } } };
});

const TEST_ICON = "data:image/svg+xml;utf8," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" rx="2" fill="#888"/></svg>');
/** The preview's icon seam (by app id). */
const icons = (byApp: Record<string, string>) => Object.assign(window, { __AIJ_APP_ICONS__: byApp });
const flush = () =>
  act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });

beforeEach(() => {
  forgetAppIconsForTests();
  setToolsForTests([...PREVIEW_TOOLS, { ...genericTool("marked"), display_name: "Marked", mark: "allowedtest" }]);
  icons({ "claude-app": TEST_ICON, "codex-app": TEST_ICON });
});
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_APP_ICONS__");
});

const logo = () => screen.getByTestId("tool-logo");

describe("ToolLogo", () => {
  it("shows a desktop app's own icon, with no badge", async () => {
    render(<ToolLogo tool="codex" size={40} />);
    await flush();
    expect(logo().querySelector("img")!.getAttribute("src")).toBe(TEST_ICON);
    expect(within(logo()).queryByTestId("tool-logo-badge")).toBeNull();
    expect(logo().style.width).toBe("40px");
  });

  it("shows a CLI as its vendor's desktop app icon with a terminal badge, at every size", async () => {
    for (const size of [24, 32, 40] as const) {
      render(<ToolLogo tool="claude-code" kind="cli" size={size} />);
      await flush();
      expect(logo().querySelector("img")!.getAttribute("src")).toBe(TEST_ICON);
      expect(within(logo()).getByTestId("tool-logo-badge").getAttribute("aria-hidden")).toBe("true");
      cleanup();
    }
  });

  it("stands a filled neutral squircle with a letter in when its app isn't on this Mac", async () => {
    icons({});
    render(<ToolLogo tool="claude-code" size={32} />);
    await flush();
    const mark = logo().querySelector("[data-mark]")!;
    expect(mark.getAttribute("data-mark")).toBe("letter");
    expect(mark.className).toContain("bg-surface-3");
    expect(mark.querySelector("[data-letters]")!.getAttribute("data-letters")).toBe("C");
  });

  it("keeps OpenCode, Kimi and Grok on their letters (no app in the catalog, no mark allowed)", async () => {
    for (const [tool, letter] of [["opencode", "O"], ["kimi-code", "K"], ["grok-build", "G"]] as const) {
      render(<ToolLogo tool={tool} kind="cli" size={24} />);
      await flush();
      expect(logo().querySelector("img")).toBeNull();
      expect(logo().querySelector("[data-letters]")!.getAttribute("data-letters")).toBe(letter);
      cleanup();
    }
  });

  it("draws a tool's mark in the squircle once its owner allows it", async () => {
    render(<ToolLogo tool="marked" size={40} />);
    await flush();
    expect(logo().querySelector("[data-mark]")!.getAttribute("data-mark")).toBe("allowedtest");
    expect(logo().querySelector("path")!.getAttribute("d")).toBe("M4 4h16v16H4z");
  });

  it("is decorative beside its name, and an image named by `alt` where no name is", async () => {
    const { rerender } = render(<ToolLogo tool="claude-code" kind="cli" />);
    await flush();
    expect(screen.queryByRole("img")).toBeNull();
    rerender(<ToolLogo tool="claude-code" kind="cli" alt="Claude Code CLI" />);
    await flush();
    expect(screen.getByRole("img", { name: "Claude Code CLI" })).toBeTruthy();
  });
});

describe("LogoTile", () => {
  it("is the same squircle with an icon, in the accent when on", () => {
    const { rerender } = render(<LogoTile icon={IconInfo} size={40} />);
    expect(screen.getByTestId("logo-tile").className).toContain("bg-surface-3");
    rerender(<LogoTile icon={IconInfo} size={40} on />);
    expect(screen.getByTestId("logo-tile").className).toContain("bg-accent-soft");
  });
});
