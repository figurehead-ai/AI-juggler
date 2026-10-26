/**
 * AJ-E1526 — marks: every tool in the catalog resolves to a mark it may
 * draw or the letter mark; an unknown tool gets the letter mark; a mark
 * is drawn only when its owner allows it; marks are decorative beside the
 * visible name, never instead of it.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { drawableMark, markLetters, ToolMark } from "./ToolMark";
import { MARKS } from "../assets/third-party-marks";
import { setToolsForTests, tools, genericTool } from "../lib/tools";
import { PREVIEW_TOOLS } from "../preview/tools";
import { AddProviderSheet } from "../screens/AddProviderSheet";

afterEach(() => {
  cleanup();
  setToolsForTests(PREVIEW_TOOLS);
});

describe("AJ-E1526 tool marks", () => {
  it("every catalog tool resolves to a drawable mark or the letter mark, and each named mark is registered", () => {
    for (const tool of tools()) {
      for (const of of ["tool", "vendor"] as const) {
        const { container } = render(<ToolMark tool={tool.id} of={of} />);
        const mark = container.firstElementChild as HTMLElement;
        expect(mark.getAttribute("aria-hidden")).toBe("true");
        expect(["letter", drawableMark(tool.id, of)?.id]).toContain(mark.dataset.mark);
        cleanup();
      }
      for (const id of [tool.mark, tool.vendor_mark]) {
        if (id) expect(MARKS[id], `mark ${id} has a registry entry`).toBeTruthy();
      }
    }
  });

  it("an unknown tool gets the letter mark", () => {
    const { container } = render(<ToolMark tool="never-heard-of" />);
    expect((container.firstElementChild as HTMLElement).dataset.mark).toBe("letter");
    // Drawn by CSS from data-letters: never text, so never a visible label.
    expect((container.firstElementChild as HTMLElement).dataset.letters).toBe("N");
    expect(container.textContent).toBe("");
  });

  it("a mark whose owner hasn't allowed it is never drawn; an allowed one is, in currentColor", () => {
    expect(drawableMark("claude-code")).toBeNull();
    MARKS.test = { owner: "Test", allowed: true, path: "M0 0h24v24H0z" };
    setToolsForTests([{ ...genericTool("acme"), mark: "test" }]);
    const { container } = render(<ToolMark tool="acme" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("fill")).toBe("currentColor");
    expect(svg.dataset.mark).toBe("test");
    delete MARKS.test;
  });

  it("the letter: the name's first, one letter as an app's fallback", () => {
    expect(markLetters("Claude Code")).toBe("C");
    expect(markLetters("Codex")).toBe("C");
    expect(markLetters("OpenAI")).toBe("O");
  });

  it("a mark sits beside the visible name, never instead of it (Add account)", async () => {
    render(<AddProviderSheet onClose={() => {}} onChoose={() => {}} />);
    const card = (await screen.findAllByTestId("provider-card"))[0]!;
    expect(card.querySelector("[data-mark]")?.getAttribute("aria-hidden")).toBe("true");
    expect(card.textContent).toContain("Claude Code");
  });
});
