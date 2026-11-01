import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ScrollingScreen } from "../components/ScreenBoundary";
import { PageShell } from "../pages/PageShell";
import { useScrollMemory } from "./scroll-memory";

/**
 * 0.3.4, the page-switching report — every return to Home, Projects or
 * Usage started at the top. A screen's scroll box now opens where it was
 * left, as soon as its content is tall enough, never against the user,
 * and at the top when what it shows changed.
 */

// jsdom doesn't lay out: each box scrolls as far as `room` says.
let room = 10_000;
const tops = new WeakMap<Element, number>();
const original = Object.getOwnPropertyDescriptor(Element.prototype, "scrollTop")!;
beforeEach(() => {
  room = 10_000;
  Object.defineProperty(Element.prototype, "scrollTop", {
    configurable: true,
    get(this: Element) {
      return tops.get(this) ?? 0;
    },
    set(this: Element, value: number) {
      tops.set(this, Math.max(0, Math.min(value, room)));
    },
  });
});
afterEach(() => {
  cleanup();
  Object.defineProperty(Element.prototype, "scrollTop", original);
});

function Box({ id, subject }: { id: string; subject?: string }) {
  const ref = useScrollMemory(id, { subject });
  return <div ref={ref} data-testid="box" />;
}

const scrollTo = (element: HTMLElement, top: number) => {
  element.scrollTop = top;
  fireEvent.scroll(element);
};

describe("useScrollMemory", () => {
  it("opens a screen again where it was left, in its first frame", () => {
    const first = render(<Box id="home" />);
    scrollTo(first.getByTestId("box"), 300);
    first.unmount();
    const again = render(<Box id="home" />);
    expect(again.getByTestId("box").scrollTop).toBe(300);
  });

  it("keeps each screen's own place", () => {
    const home = render(<Box id="home" />);
    scrollTo(home.getByTestId("box"), 300);
    home.unmount();
    const usage = render(<Box id="usage" />);
    expect(usage.getByTestId("box").scrollTop).toBe(0);
  });

  it("starts at the top when what the screen shows changed", () => {
    const first = render(<Box id="history" subject="" />);
    scrollTo(first.getByTestId("box"), 300);
    first.unmount();
    const searched = render(<Box id="history" subject="stripe" />);
    expect(searched.getByTestId("box").scrollTop).toBe(0);
    searched.unmount();
    // And the old search's place is gone with it.
    expect(render(<Box id="history" subject="" />).getByTestId("box").scrollTop).toBe(0);
  });

  it("waits for the content to be tall enough, then puts it back", async () => {
    const first = render(<Box id="usage" />);
    scrollTo(first.getByTestId("box"), 600);
    first.unmount();
    room = 200; // only the cached part is there yet
    const again = render(<Box id="usage" />);
    const box = again.getByTestId("box");
    expect(box.scrollTop).toBe(200);
    room = 10_000; // the rest arrived
    await waitFor(() => expect(box.scrollTop).toBe(600));
  });

  it("never scrolls against the user", async () => {
    const first = render(<Box id="usage" />);
    scrollTo(first.getByTestId("box"), 600);
    first.unmount();
    room = 200;
    const again = render(<Box id="usage" />);
    const box = again.getByTestId("box");
    fireEvent.wheel(box);
    room = 10_000;
    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(box.scrollTop).toBe(200);
  });

  it("lets the screen's own scrolling win (a section it opens on)", async () => {
    const first = render(<Box id="settings" />);
    scrollTo(first.getByTestId("box"), 600);
    first.unmount();
    room = 200;
    const again = render(<Box id="settings" />);
    const box = again.getByTestId("box");
    box.scrollTop = 50; // the screen scrolled to its section
    room = 10_000;
    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(box.scrollTop).toBe(50);
  });
});

describe("a box kept on screen for another subject", () => {
  it("goes to that subject's place, or the top (one session's page to the next)", () => {
    const view = render(<Box id="page:session:a" />);
    scrollTo(view.getByTestId("box"), 300);
    view.rerender(<Box id="page:session:b" />);
    expect(view.getByTestId("box").scrollTop).toBe(0);
    scrollTo(view.getByTestId("box"), 120);
    view.rerender(<Box id="page:session:a" />);
    expect(view.getByTestId("box").scrollTop).toBe(300);
  });
});

describe("where it's used", () => {
  it("a main screen (ScrollingScreen) opens where it was left", () => {
    const screen = () => (
      <ScrollingScreen screen="projects" label="Projects">
        <p>rows</p>
      </ScrollingScreen>
    );
    const first = render(screen());
    const box = first.container.firstElementChild as HTMLElement;
    scrollTo(box, 200);
    first.unmount();
    const again = render(screen());
    expect((again.container.firstElementChild as HTMLElement).scrollTop).toBe(200);
  });

  it("a page's tab opens where it was left for that item, at the top for another", () => {
    const page = (subject: string) => (
      <PageShell kind="project" subject={subject} tabs={[{ id: "overview", label: "Overview", source: "" }]} header={<h2>p</h2>} render={() => <p>body</p>} />
    );
    const first = render(page("proj-1"));
    scrollTo(first.getByTestId("page-panel-overview"), 400);
    first.unmount();
    const again = render(page("proj-1"));
    expect(again.getByTestId("page-panel-overview").scrollTop).toBe(400);
    again.unmount();
    expect(render(page("proj-2")).getByTestId("page-panel-overview").scrollTop).toBe(0);
  });
});
