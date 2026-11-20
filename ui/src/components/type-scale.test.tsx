/**
 * D-126 — the type scale's three steps and the one Section: page title
 * 22 semibold, section heading 15 semibold ink, label 12 capitals; body
 * 14, help 13 muted. Every heading takes its step's class, never its own
 * size (the pool inspector's sections were 13 px, CardTitle 14, StepHead
 * 15: three sizes for the same kind of heading).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import styles from "../styles.css?raw";
import { CardTitle } from "./CardTitle";
import { FoldSection, Section } from "./Section";
import { Section as InspectorSection } from "../pages/pool/Section";
import { StepHead } from "../screens/pools/StepHead";

afterEach(cleanup);

/** The declarations of `.name { … }` in styles.css. */
function rule(name: string): string {
  const match = styles.match(new RegExp(`\\.${name}\\s*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
}

describe("the type scale", () => {
  it("has its three steps and help as classes on the app's tokens", () => {
    expect(rule("type-title")).toContain("var(--text-title)");
    expect(rule("type-title")).toContain("font-weight: 600");
    expect(rule("type-title")).toContain("var(--color-ink)");
    expect(rule("type-section")).toContain("var(--text-section)");
    expect(rule("type-section")).toContain("font-weight: 600");
    expect(rule("type-section")).toContain("var(--color-ink)");
    expect(rule("section-label")).toContain("var(--text-label)");
    expect(rule("section-label")).toContain("text-transform: uppercase");
    expect(rule("type-help")).toContain("var(--text-meta)");
    expect(rule("type-help")).toContain("var(--color-muted)");
  });
});

describe("Section", () => {
  it("is a heading on the scale's middle step, its help beside it, its content below", () => {
    render(
      <Section title="Quiet hours" help="Notifications wait till they end" action={<button type="button">Change</button>}>
        <p>Hold notifications at night</p>
      </Section>,
    );
    const heading = screen.getByRole("heading", { name: "Quiet hours", level: 3 });
    expect(heading.className).toContain("type-section");
    expect(screen.getByText("Notifications wait till they end").className).toContain("type-help");
    expect(screen.getByRole("button", { name: "Change" })).toBeTruthy();
    expect(screen.getByText("Hold notifications at night")).toBeTruthy();
  });

  it("keeps the pool inspector's divided look, its headings now on the scale (were 13 px)", () => {
    const view = render(
      <InspectorSection title="Tasks" testId="s">
        <p>None yet</p>
      </InspectorSection>,
    );
    const heading = screen.getByRole("heading", { name: "Tasks", level: 4 });
    expect(heading.className).toContain("type-section");
    expect(heading.className).not.toContain("text-meta");
    expect(view.getByTestId("s").className).toContain("border-t");
  });

  it("folds its details, its title and line in view", () => {
    render(
      <FoldSection title="Access" line="What its sessions may reach" testId="f">
        <p>Inside</p>
      </FoldSection>,
    );
    expect(screen.getByText("What its sessions may reach")).toBeTruthy();
  });
});

describe("headings on the scale", () => {
  it("a step's head is the page's title, its why in body type", () => {
    render(<StepHead step="who" eyebrow="Step 3 of 9 · The basics" />);
    expect(screen.getByTestId("new-pool-step-title").className).toContain("type-title");
    expect(screen.getByTestId("new-pool-step-why").className).toContain("text-body");
    expect(screen.getByText("Step 3 of 9 · The basics").className).toContain("section-label");
  });

  it("a card's title is a section heading (was 14 px)", () => {
    render(<CardTitle>Updates</CardTitle>);
    const heading = screen.getByRole("heading", { name: "Updates" });
    expect(heading.className).toContain("type-section");
    expect(heading.className).not.toContain("text-sm");
  });
});
