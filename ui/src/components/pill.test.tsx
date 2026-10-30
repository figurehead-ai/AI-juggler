/** The 0.3 re-render: a sentence in a pill may wrap in a narrow column. */
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { Pill } from "./Pill";

afterEach(cleanup);

it("keeps a word on one line, and lets a sentence wrap inside its column", () => {
  const word = render(<Pill tone="neutral" testId="p">Open</Pill>).getByTestId("p").className;
  expect(word).toContain("whitespace-nowrap");
  expect(word).toContain("shrink-0");
  cleanup();
  const sentence = render(
    <Pill tone="neutral" testId="p" wrap>
      2 of 6 agent messages in a row without you
    </Pill>,
  ).getByTestId("p").className;
  expect(sentence).not.toContain("whitespace-nowrap");
  expect(sentence).toContain("max-w-full");
});
