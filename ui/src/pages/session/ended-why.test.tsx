/**
 * rc30: a session that ended with AI Juggler (a quit) read "Ended with AI
 * Juggler" in its header, its reason nowhere. The reason sits beside the
 * chip, as its ending's detail says it; a live one shows none.
 */
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { instance } from "../../test/fixtures";
import { fromInstance } from "./model";
import { SessionChips } from "./SessionHead";

afterEach(cleanup);

const session = (over: Parameters<typeof instance>[0]) => instance({ account_label: "Lab", conversation_id: "c-1", status: "stopped", started_at_ms: 1_000, last_activity_ms: 2_000, ...over });

describe("rc30 an ended session says why beside how", () => {
  it("Ended with AI Juggler, and why", () => {
    const model = fromInstance(session({ ending: { how: "with_app", detail: "AI Juggler quit while it ran" } }));
    render(<SessionChips model={model} account={undefined} now={3_000} />);
    expect(screen.getByTestId("session-state-chip").textContent).toContain("Ended with AI Juggler");
    expect(screen.getByTestId("session-ended-why").textContent).toBe("AI Juggler quit while it ran");
  });

  it("an error's detail the same way; nothing when it ended without one, or runs", () => {
    render(<SessionChips model={fromInstance(session({ ending: { how: "error", detail: "exit code 1" } }))} account={undefined} now={3_000} />);
    expect(screen.getByTestId("session-ended-why").textContent).toBe("exit code 1");
    cleanup();
    render(<SessionChips model={fromInstance(session({ ending: { how: "finished", detail: null } }))} account={undefined} now={3_000} />);
    expect(screen.queryByTestId("session-ended-why")).toBeNull();
    cleanup();
    render(<SessionChips model={fromInstance(session({ status: "running", ending: { how: "error", detail: "exit code 1" } }))} account={undefined} now={3_000} />);
    expect(screen.queryByTestId("session-ended-why")).toBeNull();
  });
});
