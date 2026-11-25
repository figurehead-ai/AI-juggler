/**
 * AJ-E1556 — a tip is read whole or not at all: every tip is the app's one
 * tip (`TipLayer`), drawn over the page (portalled, the popover layer),
 * its words wrapping, never ellipsised, and kept inside the window. The
 * 0.5.1 Nightly: Bot's New topic tip read "Nothing is do…" with a "23"
 * drawn over it. The rendered check (every tip in the gallery states,
 * measured) is `tests/visual/tooltip-overflow.spec.ts`.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { BotHead } from "./bot/BotHead";
import { WORDS } from "../lib/bot-words";
import { TIP_LOOK, keepInWindow } from "./WhyTip";
import type { AiJobInfo } from "../lib/ipc/ai";

afterEach(cleanup);

/** Whether an element or one it sits in keeps its words on one line or cuts them. */
function cuts(element: HTMLElement): string[] {
  const found: string[] = [];
  for (let at: HTMLElement | null = element; at; at = at.parentElement) {
    const classes = at.className && typeof at.className === "string" ? at.className.split(/\s+/) : [];
    for (const name of ["truncate", "whitespace-nowrap", "text-ellipsis", "line-clamp-1"]) if (classes.includes(name)) found.push(`${at.tagName.toLowerCase()}.${name}`);
  }
  return found;
}

describe("AJ-E1556 R1 Bot's New topic tip is whole, with nothing over it", () => {
  const job = { job: "chat", state: "ready", beta: false, beta_on: false } as unknown as AiJobInfo;

  it("shows on hover and focus, drawn over the page, its words whole and wrapping", () => {
    render(<BotHead job={job} onClear={() => undefined} onMemories={() => undefined} />);
    const button = screen.getByTestId("bot-new-topic");
    expect(screen.queryByTestId("bot-topic-tip")).toBeNull();
    fireEvent.pointerEnter(button);
    const tip = screen.getByTestId("bot-topic-tip");
    // Over the page: on the body, not inside the head (whose nowrap cut it,
    // and whose neighbours, the conversation's counts, were drawn over it).
    expect(tip.parentElement).toBe(document.body);
    expect(tip.getAttribute("role")).toBe("tooltip");
    expect(button.getAttribute("aria-describedby")).toBe(tip.id);
    expect(tip.textContent).toBe(`${WORDS.newTopic}${WORDS.newTopicTip}`);
    // Its words wrap; nothing it sits in keeps them on one line or cuts them.
    expect(tip.className).toContain("whitespace-normal");
    expect(tip.className).toContain("wrap-break-word");
    expect(cuts(tip)).toEqual([]);
    for (const line of tip.querySelectorAll<HTMLElement>("*")) expect(cuts(line).filter((cut) => !cut.startsWith("body"))).toEqual([]);
    // The popover layer (ELV-1: 40–50), above the panel's own layers.
    expect(tip.className).toContain("z-50");
    fireEvent.pointerLeave(button);
    expect(screen.queryByTestId("bot-topic-tip")).toBeNull();
    act(() => button.focus());
    expect(screen.getByTestId("bot-topic-tip")).toBeTruthy();
  });
});

describe("AJ-E1556 R2 the one tip's look and place", () => {
  it("wraps to its words: no cut, no ellipsis, a long word broken", () => {
    const classes = TIP_LOOK.split(" ");
    expect(classes).toEqual(expect.arrayContaining(["fixed", "z-50", "whitespace-normal", "wrap-break-word"]));
    expect(classes).not.toEqual(expect.arrayContaining(["truncate"]));
    expect(classes.some((name) => /^(text-ellipsis|line-clamp|whitespace-nowrap)/.test(name))).toBe(false);
  });

  it("is kept inside the window, 8 px in from each edge", () => {
    const tip = document.createElement("span");
    document.body.append(tip);
    const box = (left: number, top: number) => ({ left, top, right: left + 256, bottom: top + 60, width: 256, height: 60, x: left, y: top, toJSON: () => ({}) });
    const { innerWidth: w, innerHeight: h } = window;
    tip.getBoundingClientRect = () => box(w - 100, h - 20) as DOMRect;
    keepInWindow(tip);
    expect(tip.style.translate).toBe(`${-(256 - 100 + 8)}px ${-(60 - 20 + 8)}px`);
    tip.getBoundingClientRect = () => box(-30, 4) as DOMRect;
    keepInWindow(tip);
    expect(tip.style.translate).toBe("38px 4px");
    tip.getBoundingClientRect = () => box(40, 40) as DOMRect;
    keepInWindow(tip);
    expect(tip.style.translate).toBe("");
    tip.remove();
  });
});
