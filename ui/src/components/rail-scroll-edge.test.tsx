/**
 * 0.4.1 — the rail's list under its fixed head. Scrolled down, its first
 * row was cut hard against the tabs' bottom edge with no sign of more
 * above: the list now carries the scroll edge (a hairline and a soft
 * shadow, styles.css .scroll-edge) only while it's scrolled. And the
 * "new above" pill's bar has its place between the head and the list.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import css from "../styles.css?raw";
import { instance } from "../test/fixtures";
import { WorkflowRail } from "./WorkflowRail";

afterEach(cleanup);

function rail() {
  const sessions = ["s-1", "s-2", "s-3"].map((id) => instance({ id, account_id: "claude-work", git_branch: id, cwd: "/w/app" }));
  return render(
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
}

/** jsdom keeps no scroll: the list's is set by hand, then its event. */
function scrollTo(list: HTMLElement, top: number) {
  Object.defineProperty(list, "scrollTop", { configurable: true, value: top });
  act(() => void fireEvent.scroll(list));
}

describe("the rail's list under its head", () => {
  it("shows its scroll edge only while it's scrolled down", () => {
    const view = rail();
    const list = view.getByTestId("rail-list");
    expect(list.classList.contains("scroll-edge")).toBe(true);
    expect(list.hasAttribute("data-scrolled")).toBe(false);
    scrollTo(list, 40);
    expect(list.hasAttribute("data-scrolled")).toBe(true);
    scrollTo(list, 0);
    expect(list.hasAttribute("data-scrolled")).toBe(false);
  });

  it("shows the same edge at its foot only while more is below", () => {
    const view = rail();
    const list = view.getByTestId("rail-list");
    Object.defineProperty(list, "clientHeight", { configurable: true, value: 300 });
    Object.defineProperty(list, "scrollHeight", { configurable: true, value: 900 });
    scrollTo(list, 0);
    expect(list.hasAttribute("data-more")).toBe(true);
    scrollTo(list, 600);
    expect(list.hasAttribute("data-more")).toBe(false);
    const rule = /\.scroll-edge::after\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(rule).toMatch(/position:\s*sticky/);
    expect(rule).toMatch(/bottom:\s*calc\(-1 \* var\(--edge-pad, 0px\)\);/);
    // The rail's own bottom padding (pb-2) is its edge pad, so the line sits at its real foot.
    expect(list.className).toContain("pb-2");
    expect(list.className).toContain("[--edge-pad:0.5rem]");
    expect(rule).toMatch(/opacity:\s*0;/);
    expect(rule).toMatch(/margin-top:\s*-1px/);
    expect(rule).toMatch(/box-shadow:\s*var\(--shadow-edge-up\)/);
    expect(css).toMatch(/\.scroll-edge\[data-more\]::after\s*\{\s*opacity:\s*1;/);
    expect(css).toMatch(/\[data-theme="light"\]\s*\{[^}]*--shadow-edge-up:/);
  });

  it("the edge is drawn at rest invisible, in the flow zero high, in both themes, and fades only with motion", () => {
    const rule = /\.scroll-edge::before\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(rule).toMatch(/position:\s*sticky/);
    expect(rule).toMatch(/opacity:\s*0;/);
    expect(rule).toMatch(/margin-bottom:\s*-1px/);
    expect(rule).toMatch(/background:\s*var\(--color-hairline\)/);
    expect(rule).toMatch(/box-shadow:\s*var\(--shadow-edge\)/);
    expect(css).toMatch(/\.scroll-edge\[data-scrolled\]::before\s*\{\s*opacity:\s*1;/);
    // A shadow for each theme.
    expect(css).toMatch(/@theme\s*\{[^}]*--shadow-edge:/);
    expect(css).toMatch(/\[data-theme="light"\]\s*\{[^}]*--shadow-edge:/);
    // Reduced motion switches every transition off (the fade with them).
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{\s*\*, \*::before, \*::after \{[^}]*transition: none !important;/);
  });

  it("keeps a place for the new-above pill between its head and the list, with no box while empty", () => {
    const view = rail();
    const slot = view.getByTestId("rail-cue-slot");
    expect(slot.classList.contains("contents")).toBe(true);
    expect(slot.childElementCount).toBe(0);
    expect(slot.nextElementSibling).toBe(view.getByTestId("rail-list"));
  });
});
