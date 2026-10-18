import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GoToPalette, toAccelerator } from "./GoToPalette";
import type { InstanceInfo } from "../lib/bridge";

/**
 * AJ-E414 evidence (Req.4): the "Go to instance…" palette lists LIVE
 * desktop and terminal instances, routes Enter correctly (desktop →
 * window focus, terminal → panel), supports full keyboard traversal
 * (combobox a11y), and rebinds the global hotkey through the capture
 * flow. Ended instances never appear.
 */

afterEach(cleanup);
beforeEach(() => {
  localStorage.clear();
});

const NOW = 1_728_800_000_000;

function mk(id: string, kind: "desktop" | "terminal", status: string, label: string): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "fake-desktop",
    account_id: "a-1",
    account_label: label,
    model: null,
    launch_model: null,
    running_model: null,
    model_source: "unknown",
    status,
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 60_000,
    last_activity_ms: NOW,
    machine_id: "this-mac",
    cwd: "/tmp/x",
    kind,
    isolation_label: kind === "desktop" ? "Single-account app — Juggler tracks, cannot isolate" : null,
    project_id: null,
    conversation_id: null,
  };
}

const SESSIONS = [
  mk("sess-term-1", "terminal", "running", "Work Max"),
  mk("sess-desk-1", "desktop", "running", "Fake Desk"),
  mk("sess-desk-2", "desktop", "starting", "Fake Desk 2"),
  mk("sess-gone", "desktop", "finished", "Ended App"),
];

function shell(props: Partial<Parameters<typeof GoToPalette>[0]> = {}) {
  const onFocusWindow = vi.fn();
  const onOpenSession = vi.fn();
  const onHotkeyChange = vi.fn(async (_combo: string) => {});
  const view = render(
    <GoToPalette
      open
      onClose={() => {}}
      sessions={SESSIONS}
      onFocusWindow={onFocusWindow}
      onOpenSession={onOpenSession}
      hotkey="Alt+Space"
      onHotkeyChange={onHotkeyChange}
      {...props}
    />,
  );
  return { view, onFocusWindow, onOpenSession, onHotkeyChange };
}

describe("AJ-E414 Go to instance… palette (Req.4)", () => {
  it("lists live instances, desktop first; ended ones never appear", () => {
    const { view } = shell();
    const palette = view.container.querySelector('[data-testid="goto-palette"]')!;
    expect(palette).not.toBeNull();
    expect(palette.querySelector('[data-testid="goto-row-sess-desk-1"]')).not.toBeNull();
    expect(palette.querySelector('[data-testid="goto-row-sess-desk-2"]')).not.toBeNull();
    expect(palette.querySelector('[data-testid="goto-row-sess-term-1"]')).not.toBeNull();
    expect(palette.querySelector('[data-testid="goto-row-sess-gone"]')).toBeNull();
    // Desktop rows sort before terminal rows.
    const ids = Array.from(palette.querySelectorAll("li[role='option']")).map(
      (li) => li.id.replace("goto-option-", ""),
    );
    expect(ids).toEqual(["sess-desk-1", "sess-desk-2", "sess-term-1"]);
  });

  it("closed palette renders nothing", () => {
    const { view } = shell({ open: false });
    expect(view.container.querySelector('[data-testid="goto-palette"]')).toBeNull();
  });

  it("Enter on a desktop row focuses its window; on a terminal row opens its panel", () => {
    const { view, onFocusWindow, onOpenSession } = shell();
    const input = view.container.querySelector("input")!;
    // Active row 0 is desktop: Enter → window focus.
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onFocusWindow).toHaveBeenCalledWith(SESSIONS[1]);
    expect(onOpenSession).not.toHaveBeenCalled();
    // Terminal row (index 2): ArrowDown ×2 then Enter → panel.
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(input.getAttribute("aria-activedescendant")).toBe("goto-option-sess-term-1");
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onOpenSession).toHaveBeenCalledWith("sess-term-1");
  });

  it("filter narrows the list (account/provider text)", () => {
    const { view } = shell();
    const input = view.container.querySelector("input")!;
    fireEvent.change(input, { target: { value: "work" } });
    const palette = view.container.querySelector('[data-testid="goto-palette"]')!;
    expect(palette.querySelector('[data-testid="goto-row-sess-term-1"]')).not.toBeNull();
    expect(palette.querySelector('[data-testid="goto-row-sess-desk-1"]')).toBeNull();
    fireEvent.change(input, { target: { value: "nothing-matches" } });
    expect(palette.textContent).toContain("No live instances");
  });

  it("combobox a11y: aria-expanded/activedescendant/roles wired", () => {
    const { view } = shell();
    const input = view.container.querySelector("input")!;
    expect(input.getAttribute("role")).toBe("combobox");
    expect(input.getAttribute("aria-expanded")).toBe("true");
    expect(input.getAttribute("aria-controls")).toBe("goto-listbox");
    expect(input.getAttribute("aria-activedescendant")).toBe("goto-option-sess-desk-1");
    expect(view.container.querySelector("#goto-listbox")!.getAttribute("role")).toBe("listbox");
    const first = view.container.querySelectorAll("li[role='option']")[0]!;
    expect(first.getAttribute("aria-selected")).toBe("true");
  });

  it("hotkey capture: ⌥Space rebinds through the change flow; Esc cancels", async () => {
    const { view, onHotkeyChange } = shell();
    fireEvent.click(view.container.querySelector('[data-testid="goto-change-hotkey"]')!);
    const input = view.container.querySelector("input")!;
    expect(input.getAttribute("placeholder")).toContain("Press the new combo");
    fireEvent.keyDown(input, { key: " ", altKey: true });
    await vi.waitFor(() => expect(onHotkeyChange).toHaveBeenCalledWith("Alt+Space"));
    // Capture again; Escape cancels without rebinding.
    onHotkeyChange.mockClear();
    fireEvent.click(view.container.querySelector('[data-testid="goto-change-hotkey"]')!);
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onHotkeyChange).not.toHaveBeenCalled();
    expect(view.container.querySelector("input")!.getAttribute("placeholder")).toContain("Go to instance");
  });

  it("capture failures surface inline, never silently", async () => {
    const { view } = shell({
      onHotkeyChange: vi.fn(async () => {
        throw new Error("register hotkey: Alt+Space conflicts");
      }),
    });
    fireEvent.click(view.container.querySelector('[data-testid="goto-change-hotkey"]')!);
    fireEvent.keyDown(view.container.querySelector("input")!, { key: " ", altKey: true });
    await vi.waitFor(() =>
      expect(view.container.querySelector('[data-testid="goto-hotkey-error"]')?.textContent)
        .toContain("conflicts"),
    );
  });

  it("toAccelerator maps the SPEC default (⌥Space) and rejects bare modifiers", () => {
    expect(
      toAccelerator({ key: " ", ctrlKey: false, altKey: true, metaKey: false, shiftKey: false }),
    ).toBe("Alt+Space");
    expect(
      toAccelerator({ key: "Alt", ctrlKey: false, altKey: true, metaKey: false, shiftKey: false }),
    ).toBeNull();
    expect(
      toAccelerator({ key: "g", ctrlKey: true, altKey: false, metaKey: false, shiftKey: true }),
    ).toBe("Control+Shift+G");
  });
});
