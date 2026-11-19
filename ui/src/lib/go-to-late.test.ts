/**
 * 0.4.6 (the rc3c walk): "Open in Claude app" on a closed profile said
 * "Starting Claude app · aij-test… the session opens once it's up" and
 * nothing after, even once the app had quit. What it did comes after the
 * 1.5 s the command answers in, as `go-to://late` with the pending
 * answer's ticket; the toast becomes it. Another one's (a Go to it since)
 * is old news.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

const heard: Array<(event: { payload: unknown }) => void> = [];
vi.mock("./ipc/transport", async (importOriginal) => ({
  ...(await importOriginal<object>()),
  listen: (_event: string, on: (event: { payload: unknown }) => void) => {
    heard.push(on);
    return Promise.resolve(() => undefined);
  },
}));

import { GO_TO_LATE, LATE_FOR, heardLate, showGoTo } from "./go-to";
import { resetToasts, toastEntries } from "../components/Toast";

afterEach(() => resetToasts());

const STARTING = "Starting Claude app · aij-test… the session opens once it's up";

describe("an Open that answers after the limit", () => {
  it("waits on its ticket, then says what happened", () => {
    showGoTo({ outcome: STARTING, reached: "pending", ticket: "go-7" });
    const pending = toastEntries()[0]!.options;
    expect(pending.message).toBe(STARTING);
    expect(pending.testId).toBe("go-to-pending");
    // Waiting isn't a failure: no warning, and it goes on its own.
    expect(pending.tone ?? "neutral").toBe("neutral");
    expect(pending.seconds).toBe(LATE_FOR);
    expect(heard.length).toBe(1);
    heard[0]!({ payload: { outcome: "Opened the session in Claude app · aij-test", reached: "thread", ticket: "go-7" } });
    const done = toastEntries()[0]!.options;
    expect(done.message).toBe("Opened the session in Claude app · aij-test");
    expect(done.tone).toBe("success");
    expect(GO_TO_LATE).toBe("go-to://late");
  });

  it("calls a start that opened the session itself a success (the rc3d walk: it wore a warning)", () => {
    showGoTo({ outcome: STARTING, reached: "pending", ticket: "go-13" });
    heardLate({ outcome: "Started Claude app · aij-test and opened the session", reached: "started", ticket: "go-13" });
    expect(toastEntries()[0]!.options.message).toBe("Started Claude app · aij-test and opened the session");
    expect(toastEntries()[0]!.options.tone).toBe("success");
  });

  it("says when it opened another one, so the user picks it in its sidebar", () => {
    showGoTo({ outcome: STARTING, reached: "pending", ticket: "go-8" });
    heardLate({ outcome: "Started Claude app · aij-test; it opened another session: pick Echo lab in its sidebar", reached: "app", ticket: "go-8" });
    expect(toastEntries()[0]!.options.message).toContain("pick Echo lab in its sidebar");
    expect(toastEntries()[0]!.options.tone).toBe("warning");
  });

  it("never lets an older Open's answer over a newer one's toast", () => {
    showGoTo({ outcome: STARTING, reached: "pending", ticket: "go-9" });
    showGoTo({ outcome: "Brought Claude app · Work forward", reached: "app" });
    heardLate({ outcome: "Opened the session in Claude app · aij-test", reached: "thread", ticket: "go-9" });
    expect(toastEntries()[0]!.options.message).toBe("Brought Claude app · Work forward");
    // Nor a second answer to the same ticket.
    showGoTo({ outcome: STARTING, reached: "pending", ticket: "go-10" });
    heardLate({ outcome: "Opened the session in Claude app · aij-test", reached: "thread", ticket: "go-10" });
    heardLate({ outcome: "It isn't open now", reached: "none", ticket: "go-10" });
    expect(toastEntries()[0]!.options.message).toBe("Opened the session in Claude app · aij-test");
  });

  it("listens once, however many it waits on", () => {
    showGoTo({ outcome: STARTING, reached: "pending", ticket: "go-11" });
    showGoTo({ outcome: STARTING, reached: "pending", ticket: "go-12" });
    expect(heard.length).toBe(1);
  });
});
