/**
 * 0.5.1 F74 (29-door undo-toast): after a switch the page opened on the
 * new session before the list held it and read "Session · Unavailable ·
 * Nothing to show here yet". It says what happened and where the
 * conversation is now, with a skeleton, until the session arrives.
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RightInspector } from "./RightInspector";
import { expectArrival, resetArrivals } from "../lib/arriving";

afterEach(() => {
  cleanup();
  resetArrivals();
});

const NOW = 1_728_800_000_000;

function page(id: string, now = NOW) {
  return render(
    <RightInspector
      selection={{ kind: "instance", id }}
      data={{ instance: null }}
      sessions={[]}
      projects={[]}
      accounts={[]}
      profiles={[]}
      terminalTitle={null}
      freshness={{}}
      now={now}
      onOpenProject={() => {}}
      onOpenSession={() => {}}
      onLaunch={() => {}}
      onStopped={() => {}}
      onHandover={() => {}}
      onNew={() => {}}
    />,
  );
}

describe("a session a switch just made", () => {
  it("says it is starting on the target, where it came from, and loads, never Unavailable", () => {
    expectArrival("sess-new", "Personal", "Fix the flaky login test", NOW - 1_000);
    page("sess-new");
    expect(screen.getByTestId("arriving-session").textContent).toContain("Starting on Personal…");
    expect(screen.getByTestId("arriving-from").textContent).toBe("Switched from “Fix the flaky login test”. The original stays resumable.");
    expect(screen.getByTestId("arriving-skeleton").getAttribute("aria-busy")).toBe("true");
    expect(document.body.textContent).not.toMatch(/Unavailable|Nothing to show here yet/);
  });

  it("past its wait, says plainly where the conversation is instead of loading on", () => {
    expectArrival("sess-new", "Personal", null, NOW - 25_000);
    page("sess-new");
    expect(screen.getByTestId("arriving-session").textContent).toContain("On Personal");
    expect(screen.getByTestId("arriving-slow").textContent).toContain("The conversation continues on Personal.");
    expect(screen.queryByTestId("arriving-skeleton")).toBeNull();
  });

  it("an arrival for this id shows as soon as it is expected", () => {
    page("sess-late");
    expect(screen.queryByTestId("arriving-session")).toBeNull();
    act(() => expectArrival("sess-late", "Work", null, NOW));
    expect(screen.getByTestId("arriving-session").textContent).toContain("Starting on Work…");
  });
});
