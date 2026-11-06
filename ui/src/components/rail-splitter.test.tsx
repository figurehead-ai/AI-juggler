/**
 * AJ-E2909 vitest — the list's edge from the keyboard: focusable, named,
 * its width heard, ← → by 8, ⇧ by 32, Home/End to the ends, kept as a
 * drag's width is.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RailSplitter, STORE_WIDTH, WIDTH_MAX, WIDTH_MIN, readWidth, widthForKey } from "./RailSplitter";

function Host() {
  const [width, setWidth] = useState(264);
  return <RailSplitter width={width} onWidth={setWidth} controls="rail-list" />;
}

beforeEach(() => {
  localStorage.removeItem(STORE_WIDTH);
  vi.useFakeTimers();
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("AJ-E2909 — the rail's splitter", () => {
  it("is a focusable separator that says its width", () => {
    const { getByRole } = render(<Host />);
    const handle = getByRole("separator", { name: "Resize the list" });
    expect(handle.tabIndex).toBe(0);
    expect(handle.getAttribute("aria-valuenow")).toBe("264");
    expect(handle.getAttribute("aria-valuemin")).toBe(String(WIDTH_MIN));
    expect(handle.getAttribute("aria-valuemax")).toBe(String(WIDTH_MAX));
    expect(handle.getAttribute("aria-valuetext")).toBe("264 pixels wide");
    expect(handle.getAttribute("aria-controls")).toBe("rail-list");
  });

  it("moves by 8, by 32 with shift, to the ends with Home and End, and keeps it", () => {
    const { getByRole } = render(<Host />);
    const handle = getByRole("separator", { name: "Resize the list" });
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(handle.getAttribute("aria-valuenow")).toBe("272");
    fireEvent.keyDown(handle, { key: "ArrowLeft", shiftKey: true });
    expect(handle.getAttribute("aria-valuenow")).toBe("240");
    fireEvent.keyDown(handle, { key: "End" });
    expect(handle.getAttribute("aria-valuenow")).toBe(String(WIDTH_MAX));
    act(() => void vi.advanceTimersByTime(250));
    expect(readWidth()).toBe(WIDTH_MAX);
    fireEvent.keyDown(handle, { key: "Home" });
    act(() => void vi.advanceTimersByTime(250));
    expect(readWidth()).toBe(WIDTH_MIN);
  });

  it("never passes its bounds", () => {
    expect(widthForKey(WIDTH_MIN, "ArrowLeft", true)).toBe(WIDTH_MIN);
    expect(widthForKey(WIDTH_MAX, "ArrowRight", false)).toBe(WIDTH_MAX);
    expect(widthForKey(300, "a", false)).toBeNull();
  });

  it("keeps a waiting change when the window goes (a quit inside the 200 ms)", () => {
    const { getByRole } = render(<Host />);
    fireEvent.keyDown(getByRole("separator"), { key: "End" });
    expect(localStorage.getItem(STORE_WIDTH)).toBeNull();
    act(() => void window.dispatchEvent(new Event("pagehide")));
    expect(localStorage.getItem(STORE_WIDTH)).toBe(String(WIDTH_MAX));
  });

  it("says the edge moves: a tooltip and a grip (design review #41)", () => {
    const view = render(<RailSplitter width={280} onWidth={() => {}} controls="rail" />);
    const edge = view.getByTestId("rail-splitter");
    expect(edge.getAttribute("title")).toBe("Drag, or ← →");
    expect(view.getByTestId("rail-splitter-grip")).toBeTruthy();
  });
});

describe("AJ-E2909 R1 — the handle comes after the list in the Tab order", () => {
  it("is the rail's last stop, after every row of the list and its foot", async () => {
    vi.useRealTimers();
    const { WorkflowRail } = await import("./WorkflowRail");
    const { instance } = await import("../test/fixtures");
    const sessions = ["s-1", "s-2"].map((id) => instance({ id, account_id: "claude-work", git_branch: id, cwd: "/w/app" }));
    const view = render(
      <WorkflowRail
        density="comfortable"
        onCycleDensity={() => {}}
        projects={[]}
        sessions={sessions}
        loginTerminals={[]}
        selectedId={null}
        onSelectSession={() => {}}
        onSelectProject={() => {}}
        onNewSession={() => {}}
        onNewProject={() => {}}
        header={() => <button type="button">Fold</button>}
        footer={() => <button type="button">Footer</button>}
      />,
    );
    const rail = view.getByTestId("rail");
    const stops = [...rail.querySelectorAll<HTMLElement>("button, [href], input, select, textarea, [tabindex]")].filter((element) => element.tabIndex >= 0 && !(element instanceof HTMLButtonElement && element.disabled));
    const handle = view.getByRole("separator", { name: "Resize the list" });
    expect(stops.at(-1)).toBe(handle);
    const list = view.getByTestId("rail-list");
    const rows = stops.filter((element) => list.contains(element));
    expect(rows.length).toBeGreaterThan(0);
    for (const row of rows) expect(row.compareDocumentPosition(handle) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    // No positive tabindex reorders it.
    expect(stops.some((element) => element.tabIndex > 0)).toBe(false);
  });
});

