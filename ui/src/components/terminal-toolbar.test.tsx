/**
 * The 0.4 review (46): an ended session's page said "Ended" three times.
 * Its state chip says it (and when); the terminal's strip doesn't, except
 * in the terminal's own window, where there is no chip.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { TerminalToolbar, type TerminalConnection } from "./TerminalToolbar";

afterEach(cleanup);

const strip = (connection: TerminalConnection, ownWindow: boolean) =>
  render(
    <TerminalToolbar
      connection={connection}
      note={null}
      searchOpen={false}
      maximized={false}
      ownWindow={ownWindow}
      onFind={() => null}
      onCopy={() => null}
      onZoom={() => null}
      onMaximize={() => null}
      onWindow={() => null}
    />,
  );

it("says Ended only in the terminal's own window", () => {
  expect(strip("ended", false).queryByTestId("terminal-connection")).toBeNull();
  cleanup();
  expect(strip("ended", true).getByTestId("terminal-connection").textContent).toBe("Ended");
});

it("still says Live and Reconnecting on the page", () => {
  expect(strip("live", false).getByTestId("terminal-connection").textContent).toBe("Live");
  cleanup();
  expect(strip("reconnecting", false).getByTestId("terminal-connection").textContent).toBe("Reconnecting");
});
