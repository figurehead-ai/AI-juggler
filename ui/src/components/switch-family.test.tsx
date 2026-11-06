/**
 * ADR-0028: a switch moves a conversation's whole family. The done step
 * says how many moved with it and where they nest: in the app at once, or
 * once it restarts (the app reads its records only when it starts), with
 * Restart; a terminal switch says they can be continued from AI Juggler.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DoneStep, familyOf } from "./switch/DoneStep";
import { switchPreflight, type PreflightInfoData } from "../lib/ipc/switch-preflight";

const restart = vi.fn();
vi.mock("../lib/go-to", () => ({ restartPlace: (...args: unknown[]) => restart(...args) }));

afterEach(() => {
  cleanup();
  restart.mockReset();
});

async function preflight(): Promise<PreflightInfoData> {
  return switchPreflight("sess-1", "prof-2");
}

describe("the family a switch moved", () => {
  it("is nothing to say when none moved", () => {
    expect(familyOf({ session_id: "s", pid: 1, edge_id: "e", source_session_id: "a", family_moved: 0 })).toBeUndefined();
  });

  it("nests at once when the app wasn't running", async () => {
    const family = familyOf({ session_id: "s", pid: 1, edge_id: "e", source_session_id: "a", family_moved: 20, family_nests: "now", family_place: "p" })!;
    render(<DoneStep preflight={await preflight()} target="personal-max" edgeId="e" results={[]} continuedIn="cli" family={family} />);
    const line = screen.getByTestId("switch-done-family");
    expect(line.textContent).toContain("20 sessions started from it moved with it and sit under it in");
    expect(screen.queryByTestId("switch-done-family-restart")).toBeNull();
  });

  it("waits for the app's restart and offers it, confirmed, for that profile", async () => {
    const family = familyOf({ session_id: "s", pid: 1, edge_id: "e", source_session_id: "a", family_moved: 1, family_nests: "restart", family_place: "claude:/profiles/aij-test" })!;
    render(<DoneStep preflight={await preflight()} target="personal-max" edgeId="e" results={[]} continuedIn="cli" family={family} />);
    expect(screen.getByTestId("switch-done-family").textContent).toContain("1 session started from it moved with it. It sits under it in");
    fireEvent.click(screen.getByTestId("switch-done-family-restart"));
    expect(restart).toHaveBeenCalledWith("claude:/profiles/aij-test", true);
  });

  it("says the family can be continued from AI Juggler after a terminal switch", async () => {
    const family = familyOf({ session_id: "s", pid: 1, edge_id: "e", source_session_id: "a", family_moved: 3, family_nests: "" })!;
    render(<DoneStep preflight={await preflight()} target="personal-max" edgeId="e" results={[]} continuedIn="cli" family={family} />);
    expect(screen.getByTestId("switch-done-family").textContent).toBe("3 sessions started from it moved with it. Continue any of them from AI Juggler.");
  });
});
