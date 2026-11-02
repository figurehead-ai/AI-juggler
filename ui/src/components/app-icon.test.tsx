/**
 * D-38 — apps' icons: an app's own icon once read (lazily, once per app
 * across the whole UI), a quiet square until then, and a clean letter
 * mark when there is none, never cut off and never under 12 px; the
 * profile's badge beside or at the corner, never over the icon; a Codex
 * session shows ChatGPT's icon (or Codex's letters), never another
 * tool's.
 *
 * The icons here are made-up test images (`data:` URIs); no app's real
 * icon is in the repository.
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { AppIcon, appLetters, forgetAppIconsForTests, letterSize, macRadius, pixelsFor, type AppIconSize } from "./AppIcon";
import { PlaceMark } from "./PlaceMarks";
import { ProfileBadge } from "./ProfileBadge";
import { appOfConversation, appOfSession, appOfTool } from "../lib/app-of";
import { conversationItem } from "../lib/rail-items";
import { setToolsForTests } from "../lib/tools";
import { PREVIEW_TOOLS } from "../preview/tools";
import { placesConversations } from "../preview/places";

/** A made-up test image: a grey square, nobody's icon. */
const TEST_ICON = "data:image/svg+xml;utf8," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" rx="2" fill="#888"/></svg>');
const SIZES: AppIconSize[] = [16, 20, 24, 32];

const w = () => window as unknown as Record<string, unknown>;

/** The preview's icon seam, counting each app's reads. */
function seam(icons: Record<string, string>): Map<string, number> {
  const reads = new Map<string, number>();
  w().__AIJ_APP_ICONS__ = new Proxy(icons, {
    get(target, key: string) {
      reads.set(key, (reads.get(key) ?? 0) + 1);
      return target[key];
    },
  });
  return reads;
}

const flush = () => act(async () => {
  await Promise.resolve();
  await Promise.resolve();
});

beforeEach(() => {
  forgetAppIconsForTests();
  setToolsForTests(PREVIEW_TOOLS);
  delete w().__AIJ_APP_ICONS__;
});
afterEach(() => {
  cleanup();
  delete w().IntersectionObserver;
});

describe("D-38 AppIcon", () => {
  it("shows the app's own icon once read, decorative beside its name", async () => {
    seam({ terminal: TEST_ICON });
    render(<AppIcon appId="terminal" name="Terminal" size={20} />);
    expect(screen.getByTestId("app-icon").dataset.state).toBe("loading");
    await flush();
    const icon = screen.getByTestId("app-icon");
    expect(icon.dataset.state).toBe("icon");
    const img = icon.querySelector("img")!;
    expect(img.getAttribute("src")).toBe(TEST_ICON);
    expect(img.getAttribute("alt")).toBe("");
    expect(icon.getAttribute("aria-hidden")).toBe("true");
    expect(img.width).toBe(20);
  });

  it("holds a quiet, rounded square while it loads, then falls back to letters when there's none", async () => {
    seam({});
    render(<AppIcon appId="cursor" name="Cursor" size={24} />);
    const loading = screen.getByTestId("app-icon");
    expect(loading.dataset.state).toBe("loading");
    expect(loading.className).toContain("animate-pulse");
    expect(loading.style.borderRadius).toBe(`${macRadius(24)}px`);
    await flush();
    const letters = screen.getByTestId("app-icon");
    expect(letters.dataset.state).toBe("letters");
    expect(letters.dataset.letters).toBe("C");
    // Drawn by CSS, so never text that joins a control's label.
    expect(letters.textContent).toBe("");
  });

  it("reads each app once for every row that shows it", async () => {
    const reads = seam({ "claude-app": TEST_ICON });
    render(
      <>
        {Array.from({ length: 20 }, (_, n) => (
          <AppIcon key={n} appId="claude-app" name="Claude" size={20} />
        ))}
      </>,
    );
    await flush();
    expect(screen.getAllByTestId("app-icon").every((icon) => icon.dataset.state === "icon")).toBe(true);
    expect(reads.get("claude-app")).toBe(1);
    // A later row of the same app draws at once, with no read.
    cleanup();
    render(<AppIcon appId="claude-app" name="Claude" size={20} />);
    expect(screen.getByTestId("app-icon").dataset.state).toBe("icon");
    expect(reads.get("claude-app")).toBe(1);
  });

  it("asks only once the icon is on screen", async () => {
    let show: ((entries: Array<{ isIntersecting: boolean }>) => void) | null = null;
    w().IntersectionObserver = class {
      constructor(callback: (entries: Array<{ isIntersecting: boolean }>) => void) {
        show = callback;
      }
      observe() {}
      disconnect() {}
    };
    const reads = seam({ zed: TEST_ICON });
    render(<AppIcon appId="zed" name="Zed" size={16} />);
    await flush();
    expect(reads.get("zed")).toBeUndefined();
    expect(screen.getByTestId("app-icon").dataset.state).toBe("loading");
    act(() => show!([{ isIntersecting: true }]));
    await flush();
    expect(reads.get("zed")).toBe(1);
    expect(screen.getByTestId("app-icon").dataset.state).toBe("icon");
  });

  it("never cuts its letter off: one letter at every size, 12 px or more", async () => {
    seam({});
    const names = ["ChatGPT", "Claude", "Terminal", "Visual Studio Code", "iTerm", "WezTerm", "kitty", "Warp", "Mm", "WW"];
    for (const size of SIZES) {
      for (const name of names) {
        render(<AppIcon appId={`missing-${name}`} name={name} size={size} />);
        await flush();
        const mark = screen.getByTestId("app-icon");
        const letters = mark.dataset.letters!;
        const font = Number.parseFloat(mark.style.fontSize);
        expect([...letters].length, `${name} at ${size}`).toBe(1);
        expect(font, `${name} at ${size}`).toBeGreaterThanOrEqual(12);
        expect(font).toBe(letterSize(size));
        // A wide capital is about 0.72 em: the letters fit with room.
        expect(letters.length * font * 0.72, `${name} at ${size}`).toBeLessThanOrEqual(size - 2);
        expect(mark.className).not.toContain("overflow-hidden");
        expect(mark.className).toContain("justify-center");
        cleanup();
      }
    }
    expect(appLetters("Visual Studio Code", 24)).toBe("V");
    expect(appLetters("ChatGPT", 32)).toBe("C");
    expect(appLetters("ChatGPT", 16)).toBe("C");
  });

  it("draws AI Juggler's own mark for its own place when its bundle can't be read", async () => {
    seam({});
    render(<AppIcon appId="ai-juggler" name="AI Juggler" size={20} />);
    await flush();
    const mark = screen.getByTestId("app-icon");
    expect(mark.dataset.state).toBe("letters");
    expect(mark.dataset.letters).toBeUndefined();
    expect(mark.querySelector("svg")).not.toBeNull();
  });

  it("asks for the pixels the screen needs", () => {
    w().devicePixelRatio = 2;
    expect([16, 20, 24, 32].map(pixelsFor)).toEqual([32, 64, 64, 64]);
    w().devicePixelRatio = 1;
    expect([16, 20, 24, 32].map(pixelsFor)).toEqual([32, 32, 32, 32]);
    w().devicePixelRatio = 3;
    expect(pixelsFor(32)).toBe(128);
  });

  it("an image that fails to draw falls back to letters", async () => {
    seam({ warp: TEST_ICON });
    render(<AppIcon appId="warp" name="Warp" size={20} />);
    await flush();
    act(() => {
      screen.getByTestId("app-icon").querySelector("img")!.dispatchEvent(new Event("error"));
    });
    expect(screen.getByTestId("app-icon").dataset.letters).toBe("W");
  });
});

describe("D-38 ProfileBadge and PlaceMark", () => {
  it("puts the profile's dot at the icon's corner, ringed, small, never over the icon's middle", async () => {
    seam({ "claude-app": TEST_ICON });
    for (const size of [20, 24, 32]) {
      render(<PlaceMark place={{ key: "Claude|Work", app: "Claude", appId: "claude-app", kind: "desktop", profile: "Work Max", appPid: 1, host: null }} size={size} />);
      await flush();
      const corner = screen.getByTestId("place-mark-profile");
      const dot = screen.getByTestId("profile-badge");
      const diameter = Number.parseFloat(dot.style.width);
      expect(diameter).toBeGreaterThanOrEqual(6);
      expect(diameter).toBeLessThanOrEqual(8);
      expect(dot.style.boxShadow).toContain("1.5px");
      // It sits past the corner by a third of itself: it covers only the
      // corner a macOS icon leaves clear.
      expect(Number.parseFloat(corner.style.right)).toBeLessThan(0);
      expect(Number.parseFloat(corner.style.bottom)).toBeLessThan(0);
      expect(screen.getByTestId("app-icon").dataset.state).toBe("icon");
      cleanup();
    }
  });

  it("a pill names the profile at 12 px", () => {
    render(<ProfileBadge name="Personal" variant="pill" />);
    const pill = screen.getByTestId("profile-badge");
    expect(pill.textContent).toBe("Personal");
    expect(pill.className).toContain("text-label");
  });
});

describe("D-38 the app a row shows", () => {
  const conversations = placesConversations(Date.UTC(2026, 9, 25));

  it("a Codex session shows ChatGPT's icon, or Codex's letters, never Claude Code's", async () => {
    const codex = conversations.find((c) => c.tool === "codex")!;
    expect(appOfConversation(codex).appId).toBe("codex-app");
    expect(conversationItem(codex, () => null, null).place.appId).toBe("codex-app");
    // Not open anywhere: its tool's own app, with the tool's letters.
    const ended = { ...codex, live: false, places: [] };
    expect(appOfConversation(ended)).toEqual({ appId: "codex-app", appPid: null, name: "Codex" });
    expect(appOfSession({ provider: "codex" }).appId).toBe("codex-app");

    seam({ "codex-app": TEST_ICON, "claude-app": "data:image/svg+xml;utf8,claude-test" });
    render(<AppIcon {...appOfConversation(ended)} size={24} />);
    await flush();
    expect(screen.getByTestId("app-icon").dataset.app).toBe("codex-app");
    expect(screen.getByTestId("app-icon").querySelector("img")!.getAttribute("src")).toBe(TEST_ICON);
    cleanup();

    // ChatGPT not installed: Codex's own letter, named Codex (its title
    // and app), never Claude Code's.
    forgetAppIconsForTests();
    seam({});
    render(<AppIcon {...appOfConversation(ended)} size={24} />);
    await flush();
    expect(screen.getByTestId("app-icon").dataset.letters).toBe("C");
    expect(screen.getByTestId("app-icon").getAttribute("title")).toBe("Codex");
    expect(screen.getByTestId("app-icon").dataset.app).toBe("codex-app");
  });

  it("a tool's app comes from its spec, and a tool no spec names has none", () => {
    expect(appOfTool("claude-code").appId).toBe("claude-app");
    expect(appOfTool("never-heard-of")).toEqual({ appId: null, appPid: null, name: "never-heard-of" });
  });
});
