/**
 * D-126 — FlowLayout: steps beside, the form filling to 680, a 320–380
 * preview right of it, at most 1180 wide and left-aligned; under 960 the
 * preview folds above the form as a summary strip. FlowSteps: the steps
 * in groups, each value under its name, the current one marked, a done
 * one a click back. FlowFoot: Cancel left; Back, a secondary and the one
 * filled primary right.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FlowFoot, FlowLayout, FlowSteps, type FlowStep } from "./FlowLayout";
import { primariesIn } from "./primitives";

afterEach(cleanup);

const STEPS: FlowStep[] = [
  { id: "start", label: "Start", value: "From scratch", state: "done", group: "basics" },
  { id: "where", label: "Where", value: "web", state: "done", group: "basics" },
  { id: "who", label: "Who's in", state: "current", group: "basics" },
  { id: "lead", label: "Who leads", value: "No lead yet", state: "todo", group: "how" },
  { id: "review", label: "Review", state: "todo", group: "finish", disabled: true },
];
const GROUPS = [
  { id: "basics", label: "The basics" },
  { id: "how", label: "How they work", tag: "prefilled" },
  { id: "finish", label: "Finish" },
];

/** A step button by its step name (its name goes on with its value). */
const STEP = (name: string) => new RegExp(`^(\\d+ )?${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(\\s|,|$)`);

describe("FlowSteps", () => {
  it("is a nav of steps in groups, each named, its value said with it, the current one marked", () => {
    render(<FlowSteps steps={STEPS} groups={GROUPS} onPick={() => {}} />);
    const nav = screen.getByRole("navigation", { name: "Steps" });
    expect(within(nav).getByText("How they work")).toBeTruthy();
    expect(within(nav).getByText("prefilled")).toBeTruthy();
    // WCAG 2.5.3: a step's name is what it shows, the step and its value.
    const where = screen.getByRole("button", { name: "Where web, done" });
    expect(where.getAttribute("aria-describedby")).toBeNull();
    expect(screen.getByRole("button", { name: STEP("Who's in") }).getAttribute("aria-current")).toBe("step");
    expect(screen.getByRole("button", { name: STEP("Who leads") }).getAttribute("aria-current")).toBeNull();
  });

  it("its text breaks between its number, the step and its value, as its name does (axe's label-content check read \"4Who leadsNo lead yet\")", () => {
    render(<FlowSteps steps={STEPS} groups={GROUPS} onPick={() => {}} />);
    const lead = screen.getByRole("button", { name: STEP("Who leads") });
    expect(lead.textContent).toBe("4 Who leads No lead yet");
    expect(screen.getByRole("button", { name: "4 Who leads No lead yet" })).toBe(lead);
  });

  it("numbers the steps across groups and goes back to a done one; one not reachable yet can't be picked", () => {
    const pick = vi.fn();
    render(<FlowSteps steps={STEPS} groups={GROUPS} onPick={pick} />);
    expect(screen.getByRole("button", { name: STEP("Who leads") }).textContent).toContain("4");
    // The number it shows is in its name (WCAG 2.5.3, axe label-content-name-mismatch at 1024).
    expect(screen.getByRole("button", { name: /^4 Who leads\b/ })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: STEP("Where") }));
    expect(pick).toHaveBeenCalledWith("where");
    expect(screen.getByRole("button", { name: STEP("Review") }).matches(":disabled")).toBe(true);
  });
});

describe("FlowFoot", () => {
  it("puts Cancel first and the one filled primary last, its ref for focus", () => {
    const next = vi.fn();
    const ref = createRef<HTMLButtonElement>();
    const view = render(
      <FlowFoot
        ref={ref}
        cancel={{ label: "Cancel", onClick: () => {} }}
        back={{ label: "Back", onClick: () => {} }}
        secondary={{ label: "Make pool now", onClick: () => {} }}
        primary={{ label: "Next: Who leads", onClick: next }}
        note="The rest is prefilled. You can make the pool now."
      />,
    );
    const names = screen.getAllByRole("button").map((button) => button.textContent);
    expect(names).toEqual(["Cancel", "Back", "Make pool now", "Next: Who leads"]);
    expect(primariesIn(view.container)).toEqual([ref.current]);
    fireEvent.click(screen.getByRole("button", { name: "Next: Who leads" }));
    expect(next).toHaveBeenCalled();
  });
});

describe("FlowLayout", () => {
  it("lays the steps, the form (to 680), the preview (320–380) and the foot out, at most 1180 wide", () => {
    render(
      <FlowLayout
        label="New pool"
        steps={<FlowSteps steps={STEPS} onPick={() => {}} />}
        preview={<aside aria-label="Preview">diagram</aside>}
        summary={<p>web team · 3 members</p>}
        foot={<FlowFoot primary={{ label: "Next", onClick: () => {} }} />}
      >
        <p>form</p>
      </FlowLayout>,
    );
    expect(screen.getByRole("region", { name: "New pool" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Preview" })).toBeTruthy();
    expect(screen.getByTestId("flow-body").className).toContain("max-w-[1180px]");
    expect(screen.getByTestId("flow-body").className).not.toContain("mx-auto");
    expect(screen.getByTestId("flow-form").className).toContain("max-w-[680px]");
    expect(screen.getByTestId("flow-preview").className).toContain("w-[clamp(320px,28vw,380px)]");
  });

  it("folds the preview above the form as a summary strip under 960 px", () => {
    render(
      <FlowLayout preview={<aside>diagram</aside>} summary={<p>web team · 3 members</p>}>
        <p>form</p>
      </FlowLayout>,
    );
    const body = screen.getByTestId("flow-body");
    const summary = screen.getByTestId("flow-summary");
    expect(summary.className).toContain("hidden");
    expect(summary.className).toContain("max-[960px]:block");
    expect(screen.getByTestId("flow-preview").className).toContain("max-[960px]:hidden");
    expect(body.firstElementChild).toBe(summary);
  });
});
