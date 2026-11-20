/**
 * D-126 §3: the pool canvas while it's being set up — no banner over the
 * page (its question is the checklist's last item), one checklist with a
 * ring, each step's value, the open item expanded as the view's one
 * primary, "Setting up · n of 7" in the header, and the calm running view
 * once a session has joined and setup is done, or on Hide setup.
 */
import { act, cleanup, fireEvent, render, renderHook, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { primariesIn } from "../../components/primitives";
import { resetPreviewAgentBridge } from "../../lib/ipc/agent-bridge";
import { setupRows } from "../../lib/pool-setup";
import { PoolChecklist } from "./setup/PoolChecklist";
import { SetupRing } from "./SetupRing";
import { setupPhase, useSetupHidden } from "./setup-progress";
import { SETUP } from "./setup-words";
import { openPage, resetCanvasTests, seams, seedTeam } from "./canvas.test-kit";

const facts = { own: false, every: false, whoIn: true, led: false, leadNext: true };
const bridge = (seed: Record<string, unknown> | undefined) => {
  Object.assign(window, { __AIJ_AGENT_BRIDGE__: seed });
  resetPreviewAgentBridge();
};

beforeEach(() => localStorage.clear());
afterEach(() => {
  cleanup();
  bridge(undefined);
});

describe("the header's pill", () => {
  it("says how far while steps are open, and once all are done that it is, never \"Setting up · 7 of 7\" (the rc3 baseline review)", () => {
    expect(SETUP.pill(6, 7, false)).toBe("Setting up · 6 of 7");
    expect(SETUP.pill(7, 7, false)).toBe("Setup done · no session yet");
    expect(SETUP.pill(7, 7, true)).toBe("Setup done");
    // Item 9: its session held by a sealed pool is no "no session yet".
    expect(SETUP.pill(7, 7, false, "Client (sealed)")).toBe("Setup done · its sessions in Client (sealed)");
  });
});

describe("SetupRing", () => {
  it("is one image named in words, its count in it", () => {
    render(<SetupRing done={5} total={7} />);
    const ring = screen.getByRole("img", { name: "5 of 7 done" });
    expect(ring.textContent).toBe("5/7");
  });

  it("never full while something can't work, and says so", () => {
    render(<SetupRing done={7} total={7} blocked testId="ring" />);
    expect(screen.getByRole("img", { name: "Can't work yet" })).toBeTruthy();
    const arc = screen.getByTestId("ring").querySelectorAll("circle")[1]!;
    const [drawn, round] = arc.getAttribute("stroke-dasharray")!.split(" ").map(Number);
    expect(drawn!).toBeLessThan(round!);
  });

  it("beside its words, it's no image of its own", () => {
    render(<SetupRing done={1} total={6} decorative />);
    expect(screen.queryByRole("img")).toBeNull();
  });
});

describe("the checklist (D-126 §3)", () => {
  const rows = setupRows(facts, new Set());

  it("the team tools are its last item, open with the view's one primary; the rest ticked with their values and Change", () => {
    const open = vi.fn();
    const view = render(<PoolChecklist title="Set up web team" rows={rows} onOpen={open} values={{ where: "web", lead: "Claude's next session" }} tools={{ kind: "ask" }} />);
    expect(screen.getByRole("heading", { name: "Set up web team" })).toBeTruthy();
    expect(screen.getByTestId("pool-checklist-count").textContent).toBe("6 of 7 done");
    expect(screen.getByTestId("pool-checklist").textContent).toContain("One thing left before they can talk");
    const item = screen.getByTestId("pool-checklist-open");
    expect(item.textContent).toContain("Let them message each other");
    const on = within(item).getByRole("button", { name: "Turn on team tools" });
    expect(primariesIn(view.container)).toEqual([on]);
    expect(within(item).getByRole("button", { name: "Not now" })).toBeTruthy();
    expect(screen.getAllByTestId("pool-checklist-value").map((one) => one.textContent)).toEqual(["web", "Claude's next session"]);
    fireEvent.click(screen.getByRole("button", { name: /^Who leads/ }));
    expect(open).toHaveBeenCalledWith("lead");
  });

  it("Turn on answers the one question; then the item is ticked, its Change in Settings", async () => {
    bridge({ asked: false, on: false });
    const { rerender } = render(<PoolChecklist rows={rows} onOpen={() => undefined} tools={{ kind: "ask" }} />);
    fireEvent.click(screen.getByRole("button", { name: "Turn on team tools" }));
    await waitFor(() => expect(Reflect.get(window, "__AIJ_AGENT_BRIDGE_CALLS__")).toContain("answer:true"));
    rerender(<PoolChecklist rows={rows} onOpen={() => undefined} tools={{ kind: "on" }} ready={false} />);
    expect(screen.getByTestId("pool-checklist-count").textContent).toBe("7 of 7 done");
    const settings = vi.fn();
    window.addEventListener("aij:open-settings", settings);
    fireEvent.click(screen.getByRole("button", { name: /^Let them message each other, ?done ?On ?Change$/ }));
    expect(settings).toHaveBeenCalledTimes(1);
    window.removeEventListener("aij:open-settings", settings);
  });

  it("answered Not now: Turn on stays, with where they stand; can't run: why, and no button", () => {
    const { rerender } = render(<PoolChecklist rows={rows} onOpen={() => undefined} tools={{ kind: "off" }} />);
    expect(screen.getByRole("button", { name: "Turn on team tools" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Not now" })).toBeNull();
    rerender(<PoolChecklist rows={rows} onOpen={() => undefined} tools={{ kind: "unsafe", folder: "~/Downloads" }} />);
    expect(screen.queryByRole("button", { name: "Turn on team tools" })).toBeNull();
    expect(screen.getByTestId("pool-checklist-tools-line").textContent).toContain("~/Downloads");
  });

  it("who's in not set: it's the open item, Add a member its primary", () => {
    const open = vi.fn();
    const view = render(<PoolChecklist rows={setupRows({ ...facts, whoIn: false }, new Set())} onOpen={open} tools={{ kind: "ask" }} />);
    const add = within(screen.getByTestId("pool-checklist-open")).getByRole("button", { name: "Add a member" });
    expect(primariesIn(view.container)).toEqual([add]);
    fireEvent.click(add);
    expect(open).toHaveBeenCalledWith("who");
  });
});

describe("the open item stays in view (the coordinator's critique: Turn on under the panel's foot at 1280 × 800)", () => {
  afterEach(() => vi.restoreAllMocks());

  it("the panel scrolls the open item whole into view as the checklist opens", () => {
    // A panel 300 px tall; the open item, last, drawn at 400 to 560.
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const [top, bottom] = this.hasAttribute("data-canvas-scroll") ? [0, 300] : this.dataset.testid === "pool-checklist-open" ? [400, 560] : [0, 0];
      return { top, bottom, left: 0, right: 100, width: 100, height: bottom - top, x: 0, y: top, toJSON: () => ({}) };
    });
    const view = render(
      <div data-canvas-scroll>
        <PoolChecklist rows={setupRows(facts, new Set())} onOpen={() => undefined} tools={{ kind: "ask" }} />
      </div>,
    );
    expect(view.getByTestId("pool-checklist-open").textContent).toContain("Let them message each other");
    expect(view.container.querySelector<HTMLElement>("[data-canvas-scroll]")!.scrollTop).toBe(272);
  });

  it("the panel's foot is in the list's flow, under the items, never fixed over them", async () => {
    resetCanvasTests();
    bridge({ asked: false, on: false });
    seedTeam();
    const view = await openPage();
    const foot = await view.findByTestId("pool-overview-foot");
    expect(view.getByTestId("inspector-body").contains(foot)).toBe(true);
    expect(view.getByTestId("inspector-body").lastElementChild).toBe(foot);
  });
});

describe("setting up or running", () => {
  it("running once a session has joined and setup is done, or on Hide setup", () => {
    expect(setupPhase({ joined: false, complete: true, hidden: false })).toBe("setup");
    expect(setupPhase({ joined: true, complete: false, hidden: false })).toBe("setup");
    expect(setupPhase({ joined: true, complete: true, hidden: false })).toBe("running");
    expect(setupPhase({ joined: false, complete: false, hidden: true })).toBe("running");
  });

  it("Hide setup is kept per pool on this Mac", () => {
    const { result } = renderHook(() => useSetupHidden("pool-a"));
    act(() => result.current[1](true));
    expect(renderHook(() => useSetupHidden("pool-a")).result.current[0]).toBe(true);
    expect(renderHook(() => useSetupHidden("pool-b")).result.current[0]).toBe(false);
  });
});

describe("the pool page while it's set up (D-126 §3)", () => {
  beforeEach(resetCanvasTests);

  it("no banner over the page; the header says how far; the checklist holds the one primary; Hide setup is the calm view", async () => {
    bridge({ asked: false, on: false });
    seedTeam();
    const view = await openPage();
    await waitFor(() => expect(view.getByTestId("pool-header-setup").textContent).toBe("Setting up · 6 of 7"));
    // The question isn't a banner over the page (X2 §4c): it's the last item.
    expect(view.queryByTestId("pool-team-tools")).toBeNull();
    expect(within(view.getByTestId("pool-checklist-open")).getByRole("button", { name: "Turn on team tools" })).toBeTruthy();
    expect(primariesIn(view.container)).toHaveLength(1);
    fireEvent.click(view.getByRole("button", { name: "Hide setup" }));
    expect(await view.findByTestId("pool-running")).toBeTruthy();
    expect(view.queryByTestId("pool-header-setup")).toBeNull();
    // Hidden but not done: it says how far, and Edit setup brings it back.
    expect(view.getByTestId("pool-setup-hidden").textContent).toBe("Setup hidden: 6 of 7 done.");
    fireEvent.click(view.getByRole("button", { name: "Edit setup" }));
    expect(await view.findByTestId("pool-checklist")).toBeTruthy();
  });

  it("SK-7: \"Use a template\" stays one click away while it's set up, members in or not; gone once it runs", async () => {
    bridge({ asked: false, on: false });
    seedTeam();
    const view = await openPage();
    await waitFor(() => expect(view.getByTestId("pool-header-setup").textContent).toBe("Setting up · 6 of 7"));
    // Two sessions are in it: the door used to go once a member joined.
    // It has an edge, as Add a member has (the visual review: a ghost read
    // as a label in light).
    expect(view.getByTestId("canvas-use-template").className).toBe(view.getByTestId("canvas-add-member").className);
    fireEvent.click(view.getByTestId("canvas-use-template"));
    expect(await view.findByTestId("new-pool")).toBeTruthy();
    cleanup();
    bridge({ asked: true, on: true });
    seedTeam();
    const running = await openPage();
    await running.findByTestId("pool-running");
    expect(running.queryByTestId("canvas-use-template")).toBeNull();
    // Edit setup with every step done: the checklist again, still no
    // template beside a done setup.
    fireEvent.click(running.getByRole("button", { name: "Edit setup" }));
    await waitFor(() => expect(running.getByTestId("pool-header-setup").textContent).toBe("Setup done"));
    expect(running.getByTestId("canvas-add-member")).toBeTruthy();
    expect(running.queryByTestId("canvas-use-template")).toBeNull();
  });

  it("a session in it and every step done (the team tools on): the calm view, on its own", async () => {
    bridge({ asked: true, on: true });
    seedTeam();
    seams().__AIJ_POOL_CANVAS__ = { "pool-team": { stream: [] } };
    const view = await openPage();
    const running = await view.findByTestId("pool-running");
    expect(view.queryByTestId("pool-checklist")).toBeNull();
    expect(view.queryByTestId("pool-header-setup")).toBeNull();
    expect([...running.querySelectorAll("dt")].map((term) => term.textContent)).toEqual(["Now", "Tasks", "Lead", "On their own", "Messages this week"]);
    expect(within(running).getByText("Work together")).toBeTruthy();
    expect(primariesIn(view.container)).toHaveLength(0);
  });
});
