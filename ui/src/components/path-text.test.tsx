/** 0.4.6 RC walk (A1): a path in a sentence breaks only after a "/". */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PathText } from "./PathText";

afterEach(cleanup);

describe("PathText", () => {
  it("reads as the whole path, with a break chance after each slash and none inside a name", () => {
    const { container } = render(<PathText path="~/Library/Application Support/ClaudeProfiles" />);
    const span = container.querySelector("[data-path]")!;
    expect(span.textContent).toBe("~/Library/Application Support/ClaudeProfiles");
    expect(span.querySelectorAll("wbr")).toHaveLength(3);
    expect(span.className).not.toContain("break-all");
  });
});
