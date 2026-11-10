/**
 * rc24 walk K1: a page that goes with its terminal focused takes the
 * caret with it, blurring the field while it is still on the page (so
 * macOS's input-source mark goes too, instead of staying over the next
 * page).
 */
import { cleanup, render } from "@testing-library/react";
import { useRef, type JSX } from "react";
import { afterEach, expect, it } from "vitest";
import { useCaretLeavesWith } from "./caret-leaves";

afterEach(cleanup);

function Field(): JSX.Element {
  const ref = useRef<HTMLDivElement | null>(null);
  useCaretLeavesWith(ref);
  return (
    <div ref={ref}>
      <textarea aria-label="terminal input" />
    </div>
  );
}

it("blurs its focused field while the field is still on the page", () => {
  const view = render(<Field />);
  const field = view.getByLabelText("terminal input");
  field.focus();
  expect(document.activeElement).toBe(field);
  const blurs: boolean[] = [];
  field.addEventListener("blur", () => blurs.push(field.isConnected));
  view.unmount();
  expect(blurs).toEqual([true]);
});

it("leaves focus elsewhere alone", () => {
  const outside = document.createElement("button");
  document.body.append(outside);
  outside.focus();
  const view = render(<Field />);
  view.unmount();
  expect(document.activeElement).toBe(outside);
  outside.remove();
});
