/**
 * AJ-E2803 R4: the window's title says the screen, "AI Juggler · Projects".
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { useWindowTitle, windowTitle } from "./window-title";

afterEach(cleanup);

function Shows({ screen }: { screen: string }) {
  useWindowTitle(screen);
  return null;
}

describe("the window's title (AJ-E2803 R4)", () => {
  it("is AI Juggler and the screen, and follows it", () => {
    expect(windowTitle("Projects")).toBe("AI Juggler · Projects");
    expect(windowTitle("")).toBe("AI Juggler");
    const view = render(<Shows screen="Projects" />);
    expect(document.title).toBe("AI Juggler · Projects");
    view.rerender(<Shows screen="Tasks" />);
    expect(document.title).toBe("AI Juggler · Tasks");
  });
});
