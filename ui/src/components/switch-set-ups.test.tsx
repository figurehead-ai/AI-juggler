import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SwitchSetUpsCard } from "./SwitchSetUpsCard";
import type { SetUpInfo, TurnOnInfo } from "../lib/ipc/switchSetup";

/**
 * Review of #132 (M3): set-ups outlive the switch screen. Settings lists
 * each one still in place with its Undo; with none, nothing shows.
 */

type Seams = { __AIJ_SWITCH_SET_UPS__?: SetUpInfo[]; __AIJ_SWITCH_TURN_ONS__?: TurnOnInfo[] };

// "r-stale" was changed since it was set up, so its Undo is refused.
vi.mock("../lib/ipc/switchSetup", async (actual) => {
  const real = await actual<typeof import("../lib/ipc/switchSetup")>();
  return {
    ...real,
    switchSetupUndo: (record: string) =>
      record === "r-stale"
        ? Promise.reject(new Error("It has changed since it was set up, so it's left as it is."))
        : real.switchSetupUndo(record),
  };
});

afterEach(() => {
  cleanup();
  delete (window as never as Seams).__AIJ_SWITCH_SET_UPS__;
  delete (window as never as Seams).__AIJ_SWITCH_TURN_ONS__;
});

describe("set up by switches", () => {
  it("lists each set-up with its own Undo, and undoes it", async () => {
    (window as never as Seams).__AIJ_SWITCH_SET_UPS__ = [
      { record: "r-1", place: "Personal", what: "the skill release-notes", at_ms: 2 },
      { record: "r-2", place: "Codex", what: "the MCP server github", at_ms: 1 },
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

  it("offers Forget only for a set-up whose Undo was refused, and forgets it", async () => {
    (window as never as Seams).__AIJ_SWITCH_SET_UPS__ = [
      { record: "r-stale", place: "Codex", what: "the MCP server github", at_ms: 2 },
      { record: "r-ok", place: "Personal", what: "the skill release-notes", at_ms: 1 },
    ];
    const view = render(<SwitchSetUpsCard />);
    await waitFor(() => expect(view.getByTestId("switch-set-ups")).toBeTruthy());
    expect(view.queryByRole("button", { name: /^Forget/ })).toBeNull();
    fireEvent.click(view.getByRole("button", { name: "Undo: the MCP server github on Codex" }));
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("changed since"));
    expect(view.queryByRole("button", { name: "Forget: the skill release-notes on Personal" })).toBeNull();
    fireEvent.click(view.getByRole("button", { name: "Forget: the MCP server github on Codex" }));
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("Forgot"));
    expect(view.queryByText("the MCP server github")).toBeNull();
    expect(view.queryByRole("alert")).toBeNull();
    expect(view.getByText("the skill release-notes")).toBeTruthy();
  });

  it("shows nothing when there is nothing to undo", async () => {
    (window as never as Seams).__AIJ_SWITCH_SET_UPS__ = [];
    (window as never as Seams).__AIJ_SWITCH_TURN_ONS__ = [];
    const view = render(<SwitchSetUpsCard />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(view.queryByTestId("switch-set-ups")).toBeNull();
  });

  it("keeps what an account still has to turn on, with its steps, until Check again finds it", async () => {
    const seams = window as never as Seams;
    seams.__AIJ_SWITCH_SET_UPS__ = [];
    seams.__AIJ_SWITCH_TURN_ONS__ = [
      {
        id: "t-1",
        account: "Personal",
        kind: "skill",
        name: "computer-use",
        get_it: {
          steps: ["Sign in to claude.ai as Personal.", "Start a session on Personal so it syncs."],
          open_url: "https://claude.ai/customize/skills",
          needs: ["Needs a Pro or Max plan."],
          then: null,
        },
        at_ms: 1,
      },
    ];
    const view = render(<SwitchSetUpsCard />);
    await waitFor(() => expect(view.getByTestId("switch-turn-ons")).toBeTruthy());
    expect(view.getByText("Needs a Pro or Max plan.")).toBeTruthy();
    fireEvent.click(view.getByRole("button", { name: "Check again for computer-use" }));
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("doesn't have computer-use yet"));
    // Personal's next session brought it: Check again clears it.
    seams.__AIJ_SWITCH_TURN_ONS__ = [];
    fireEvent.click(view.getByRole("button", { name: "Check again for computer-use" }));
    await waitFor(() => expect(view.queryByTestId("switch-turn-ons")).toBeNull());
    expect(view.getByRole("status").textContent).toBe("Personal has computer-use now.");
  });

  it("stops reminding when asked", async () => {
    const seams = window as never as Seams;
    seams.__AIJ_SWITCH_SET_UPS__ = [];
    seams.__AIJ_SWITCH_TURN_ONS__ = [
      { id: "t-2", account: "Personal", kind: "plugin", name: "notes", get_it: { steps: ["a"], open_url: null, needs: [], then: null }, at_ms: 1 },
    ];
    const view = render(<SwitchSetUpsCard />);
    await waitFor(() => expect(view.getByTestId("switch-turn-ons")).toBeTruthy());
    fireEvent.click(view.getByRole("button", { name: "Stop reminding me about notes on Personal" }));
    await waitFor(() => expect(view.getByRole("status").textContent).toBe("Stopped reminding you about notes"));
    expect(view.queryByTestId("switch-turn-ons")).toBeNull();
  });
});
