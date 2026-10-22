import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { SwitchSetUpsCard } from "./SwitchSetUpsCard";
import type { SetUpInfo } from "../lib/ipc/switchSetup";

/**
 * Review of #132 (M3): set-ups outlive the switch screen. Settings lists
 * each one still in place with its Undo; with none, nothing shows.
 */

type Seams = { __AIJ_SWITCH_SET_UPS__?: SetUpInfo[] };

afterEach(() => {
  cleanup();
  delete (window as never as Seams).__AIJ_SWITCH_SET_UPS__;
});

describe("set up by switches", () => {
  it("lists each set-up with its own Undo, and undoes it", async () => {
    (window as never as Seams).__AIJ_SWITCH_SET_UPS__ = [
      { record: "r-1", account: "Personal", what: "the skill release-notes", at_ms: 2 },
      { record: "r-2", account: "Codex", what: "the MCP server github", at_ms: 1 },
    ];
    const view = render(<SwitchSetUpsCard />);
    await waitFor(() => expect(view.getByTestId("switch-set-ups")).toBeTruthy());
    expect(view.getByText("the skill release-notes")).toBeTruthy();
    fireEvent.click(
      view.getByRole("button", { name: "Undo: the MCP server github on Codex" }),
    );
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("Removed"));
    expect(view.queryByText("the MCP server github")).toBeNull();
    expect(view.getByText("the skill release-notes")).toBeTruthy();
  });

  it("shows nothing when there is nothing to undo", async () => {
    (window as never as Seams).__AIJ_SWITCH_SET_UPS__ = [];
    const view = render(<SwitchSetUpsCard />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(view.queryByTestId("switch-set-ups")).toBeNull();
  });
});
