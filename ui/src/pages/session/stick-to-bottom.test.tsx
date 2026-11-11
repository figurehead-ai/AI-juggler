/**
 * 0.3.5 (ADR-0024) vitest — a conversation's scroll: at the bottom, new
 * words keep it there; scrolled up, nothing moves and a pill offers the
 * way down; earlier turns put above keep the turn being read in place.
 * jsdom has no layout: each element's geometry is set by hand.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useRef, type JSX } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useStickToBottom } from "./useStickToBottom";
import { DrivenItems } from "./DrivenPanel";

afterEach(cleanup);

const ROW = 100;
const VIEW = 300;

/** A box of `ids` rows, ROW tall each, VIEW tall itself. */
function Box({ ids }: { ids: string[] }): JSX.Element {
  const box = useRef<HTMLDivElement | null>(null);
  const { onScroll, pill, toBottom } = useStickToBottom(box, { firstId: ids[0] ?? null, lastKey: ids[ids.length - 1] ?? null });
  const attach = (element: HTMLDivElement | null) => {
    if (element && !("geometry" in element.dataset)) {
      element.dataset.geometry = "1";
      let top = 0;
      Object.defineProperty(element, "scrollTop", { get: () => top, set: (value: number) => (top = Math.max(0, Math.min(value, element.scrollHeight - VIEW))) });
      Object.defineProperty(element, "clientHeight", { get: () => VIEW });
      Object.defineProperty(element, "scrollHeight", { get: () => element.querySelectorAll("[data-turn-id]").length * ROW });
    }
    box.current = element;
  };
  return (
    <div>
      <div ref={attach} onScroll={onScroll} data-testid="box">
        {ids.map((id, index) => (
          <div
            key={id}
            data-turn-id={id}
            ref={(row) => {
              if (!row) return;
              Object.defineProperty(row, "offsetTop", { get: () => index * ROW, configurable: true });
              Object.defineProperty(row, "offsetHeight", { get: () => ROW, configurable: true });
            }}
          />
        ))}
      </div>
      {pill && <button type="button" onClick={toBottom} data-testid="pill" />}
    </div>
  );
}

const ids = (from: number, to: number) => Array.from({ length: to - from }, (_, index) => `t${from + index}`);

describe("0.3.5 the conversation's scroll", () => {
  it("opens at the bottom, and stays there as words come while the reader is there", () => {
    const view = render(<Box ids={ids(0, 10)} />);
    const box = screen.getByTestId("box");
    expect(box.scrollTop).toBe(700);
    view.rerender(<Box ids={ids(0, 12)} />);
    expect(box.scrollTop).toBe(900);
    expect(screen.queryByTestId("pill")).toBeNull();
  });

  it("scrolled up, nothing moves when words come: a pill takes the reader down", () => {
    const view = render(<Box ids={ids(0, 10)} />);
    const box = screen.getByTestId("box");
    box.scrollTop = 200;
    fireEvent.scroll(box);
    view.rerender(<Box ids={ids(0, 12)} />);
    expect(box.scrollTop).toBe(200);
    act(() => screen.getByTestId("pill").click());
    expect(box.scrollTop).toBe(900);
    expect(screen.queryByTestId("pill")).toBeNull();
  });

  it("earlier turns put above keep the turn being read where it was", () => {
    const view = render(<Box ids={ids(10, 20)} />);
    const box = screen.getByTestId("box");
    // Reading t13, 50 px into the view.
    box.scrollTop = 250;
    fireEvent.scroll(box);
    view.rerender(<Box ids={ids(0, 20)} />);
    // t13 is now at 1300: it sits at the same place on screen.
    expect(box.scrollTop).toBe(1300 - 50);
    expect(screen.queryByTestId("pill")).toBeNull();
  });
});

/** rc27 live pass 2: a driven session's page stayed put as its turns came
 * in. It follows the newest unless the reader scrolled up. */
describe("a driven session's recent conversation", () => {
  const tops = new WeakMap<HTMLElement, number>();
  const restore: Array<() => void> = [];
  const define = (name: "scrollTop" | "scrollHeight" | "clientHeight", descriptor: PropertyDescriptor) => {
    const before = Object.getOwnPropertyDescriptor(HTMLOListElement.prototype, name);
    Object.defineProperty(HTMLOListElement.prototype, name, { configurable: true, ...descriptor });
    restore.push(() => (before ? Object.defineProperty(HTMLOListElement.prototype, name, before) : Reflect.deleteProperty(HTMLOListElement.prototype, name)));
  };
  beforeEach(() => {
    define("scrollHeight", { get(this: HTMLElement) { return this.querySelectorAll("li").length * ROW; } });
    define("clientHeight", { get: () => VIEW });
    define("scrollTop", {
      get(this: HTMLElement) { return tops.get(this) ?? 0; },
      set(this: HTMLElement, value: number) { tops.set(this, Math.max(0, Math.min(value, this.scrollHeight - VIEW))); },
    });
  });
  afterEach(() => restore.splice(0).forEach((undo) => undo()));
  const items = (n: number) => Array.from({ length: n }, (_, i) => ({ kind: "agent_message", text: `line ${i}`, at_ms: i }));

  it("follows the newest output, and stays put once the reader scrolls up", () => {
    const view = render(<DrivenItems items={items(10)} streaming={null} />);
    const list = screen.getByTestId("driven-items");
    expect(list.scrollTop).toBe(700);
    view.rerender(<DrivenItems items={items(12)} streaming={null} />);
    expect(list.scrollTop).toBe(900);
    view.rerender(<DrivenItems items={items(12)} streaming="Thinking" />);
    expect(list.scrollTop).toBe(1000);
    list.scrollTop = 200;
    fireEvent.scroll(list);
    view.rerender(<DrivenItems items={items(14)} streaming={null} />);
    expect(list.scrollTop).toBe(200);
    act(() => screen.getByTestId("driven-new").click());
    expect(list.scrollTop).toBe(1100);
  });
});
