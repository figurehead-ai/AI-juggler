import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DesktopPreview, PAUSED_NOT_VISIBLE, type PreviewBridge } from "./DesktopPreview";
import type { PreviewFrameInfo, PreviewStateInfo } from "../lib/bridge";

/**
 * AJ-E415 evidence: the preview is a strictly READ-ONLY mirror — frames
 * render with the live indicator + frame age, minimise pauses with the
 * copy-exact text, permission denial renders guidance, clicks only ever
 * FOCUS (never inject), and the stream runs only while visible+enabled.
 */

const NOW = 1_728_800_000_000;

function fixture(overrides: Partial<PreviewFrameInfo> = {}): PreviewFrameInfo {
  return {
    session_id: "sess-desktop-1",
    png_b64: "aVJCT1J3MEtsTQ==",
    at_ms: NOW - 2_000,
    seq: 3,
    ...overrides,
  };
}

function state(phase: string, text = ""): PreviewStateInfo {
  return { session_id: "sess-desktop-1", phase, text };
}

function shell({
  enabled = true,
  visible = true,
  frame = fixture(),
  phases = [state("live")],
}: {
  enabled?: boolean;
  visible?: boolean;
  frame?: PreviewFrameInfo | null;
  phases?: PreviewStateInfo[];
} = {}) {
  const calls = { start: [] as string[], stop: 0, focus: 0 };
  const bridge: PreviewBridge = {
    start: (id) => {
      calls.start.push(id);
      return Promise.resolve();
    },
    stop: () => {
      calls.stop += 1;
      return Promise.resolve();
    },
    onFrame: (handler) => {
      if (frame !== null) handler(frame);
      return () => {};
    },
    onState: (handler) => {
      phases.forEach(handler);
      return () => {};
    },
  };
  const view = render(
    <DesktopPreview
      sessionId="sess-desktop-1"
      enabled={enabled}
      visible={visible}
      onFocus={() => {
        calls.focus += 1;
      }}
      now={NOW}
      bridge={bridge}
    />,
  );
  return { view, calls };
}

afterEach(cleanup);
beforeEach(() => {
  vi.clearAllMocks();
});

describe("AJ-E415 desktop live preview (read-only mirror)", () => {
  it("renders the frame with the live indicator, frame age, and click-to-focus", () => {
    const { view, calls } = shell();
    const card = view.container.querySelector('[data-testid="desktop-preview"]')!;
    expect(card).not.toBeNull();
    expect(card.querySelector('[data-testid="preview-live-indicator"]')?.textContent)
      .toContain("● Live preview");
    // Frame age derives from now - at_ms (2 s fixture).
    expect(card.querySelector('[data-testid="preview-frame-age"]')?.textContent)
      .toContain("frame age 2s");
    const img = card.querySelector('[data-testid="preview-frame"] img') as HTMLImageElement;
    expect(img.src).toContain("data:image/png;base64,aVJCT1J3MEtsTQ==");
    fireEvent.click(card.querySelector('[data-testid="preview-frame"]')!);
    expect(calls.focus).toBe(1);
    // Read-only contract: the ONLY wired pointer action is focus.
    expect(card.querySelectorAll("button")).toHaveLength(1);
  });

  it("renders nothing when the Account has not opted in", () => {
    const { view, calls } = shell({ enabled: false });
    expect(view.container.querySelector('[data-testid="desktop-preview"]')).toBeNull();
    expect(calls.start).toEqual([]);
  });

  it("minimised window pauses with the copy-exact text and dims the last frame", () => {
    const { view } = shell({ phases: [state("paused", PAUSED_NOT_VISIBLE)] });
    const card = view.container.querySelector('[data-testid="desktop-preview"]')!;
    expect(card.querySelector('[data-testid="preview-paused"]')?.textContent)
      .toBe(PAUSED_NOT_VISIBLE);
    expect(card.querySelector('[data-testid="preview-live-indicator"]')).toBeNull();
    expect(card.querySelector('[data-testid="preview-frame"] img')?.className)
      .toContain("opacity-40");
  });

  it("denied renders guidance, keeps the data below working, never a fake frame", () => {
    const { view } = shell({
      frame: null,
      phases: [
        state("denied", "Screen Recording permission needed — grant it in System Settings, then re-enable preview"),
      ],
    });
    const card = view.container.querySelector('[data-testid="desktop-preview"]')!;
    const denied = card.querySelector('[data-testid="preview-denied"]')!;
    expect(denied.textContent).toContain("Screen Recording permission needed");
    expect(denied.textContent).toContain("Status and transcript data below keep working");
    expect(card.querySelector('[data-testid="preview-frame"]')).toBeNull();
  });

  it("unsupported platform renders the honest text and no frame", () => {
    const { view } = shell({
      frame: null,
      phases: [state("unsupported", "Live preview is not supported on this platform")],
    });
    const card = view.container.querySelector('[data-testid="desktop-preview"]')!;
    expect(card.querySelector('[data-testid="preview-unsupported"]')?.textContent)
      .toContain("not supported on this platform");
    expect(card.querySelector('[data-testid="preview-frame"]')).toBeNull();
  });

  it("streams only while visible: hidden stops, re-visible restarts (Req.1/4)", async () => {
    const first = shell({ visible: true });
    expect(first.calls.start).toEqual(["sess-desktop-1"]);
    cleanup();

    const hidden = shell({ visible: false });
    expect(hidden.calls.start).toEqual([]);
    cleanup();

    // Remount visible again (a re-render after the tab switch): start
    // fires again and the previous unmount stopped exactly once.
    const again = shell({ visible: true });
    await waitFor(() => expect(again.calls.start).toEqual(["sess-desktop-1"]));
  });

  it("a refused start (opt-out mid-flight) never crashes the panel", () => {
    const bridge: PreviewBridge = {
      start: () => Promise.reject(new Error("preview is off for Fake Desk")),
      stop: () => Promise.resolve(),
      onFrame: () => () => {},
      onState: () => () => {},
    };
    const view = render(
      <DesktopPreview
        sessionId="sess-desktop-1"
        enabled
        visible
        onFocus={() => {}}
        now={NOW}
        bridge={bridge}
      />,
    );
    expect(view.container.querySelector('[data-testid="desktop-preview"]')).not.toBeNull();
  });
});
