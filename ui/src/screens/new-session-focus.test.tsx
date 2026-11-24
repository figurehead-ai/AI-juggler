import { act, cleanup, render } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { useStartFocus } from "./new-session-focus";

function Sheet({ ready }: { ready: boolean }) {
  const sheet = useRef<HTMLDivElement | null>(null);
  const start = useRef<HTMLButtonElement | null>(null);
  useStartFocus(sheet, start, ready, "button:not(:disabled), input");
  return (
    <div ref={sheet}>
      <input aria-label="folder" />
      <button ref={start} disabled={!ready}>
        Start
      </button>
    </div>
  );
}

const frame = () => act(() => new Promise<void>((done) => requestAnimationFrame(() => done())));

afterEach(cleanup);

describe("useStartFocus (AJ-E804, 0.5.1 F49)", () => {
  it("opens onto Start when it can", async () => {
    const view = render(<Sheet ready />);
    await frame();
    expect(document.activeElement).toBe(view.getByText("Start"));
  });

  it("opened before its App is read: the first control, then Start once it can take it", async () => {
    const view = render(<Sheet ready={false} />);
    await frame();
    expect(document.activeElement).toBe(view.getByLabelText("folder"));
    view.rerender(<Sheet ready />);
    expect(document.activeElement).toBe(view.getByText("Start"));
  });

  it("leaves focus where the person moved it meanwhile", async () => {
    const view = render(
      <>
        <Sheet ready={false} />
      </>,
    );
    await frame();
    const other = document.createElement("button");
    view.container.querySelector("div")!.appendChild(other);
    other.focus();
    view.rerender(<Sheet ready />);
    expect(document.activeElement).toBe(other);
  });
});
