/**
 * D-96 C (AJ-E3307 D2) — an ended run says how it ended, from its
 * session's `ending` (its process's exit, the user's Stop), the same
 * on its rail row and its page's header; nothing said, only "Ended".
 */
import { describe, expect, it } from "vitest";
import { endedWord, endedWords } from "./ended-how";
import { sessionState } from "./rail-items";
import { fromInstance } from "../pages/session/model";
import { instance } from "../test/fixtures";

describe("D-96 C — how it ended", () => {
  it("words each ending, and only \"Ended\" for none", () => {
    expect(["finished", "error", "stopped", "crashed", undefined, "something-new"].map(endedWord)).toEqual(["Finished", "Error", "Stopped by you", "Crashed", "Ended", "Ended"]);
    expect(endedWords("error", "exit code 1")).toBe("Error · exit code 1");
  });

  it("the rail's row and the page's header say the same", () => {
    const crashed = instance({ status: "stopped", ending: { how: "crashed", detail: "signal 11" } });
    const yours = instance({ status: "stopped", ending: { how: "stopped", detail: null } });
    const done = instance({ status: "finished", ending: { how: "finished", detail: null } });
    const unsaid = instance({ status: "stopped" });
    expect([crashed, yours, done, unsaid].map((row) => sessionState(row).word)).toEqual(["Crashed", "Stopped by you", "Finished", "Ended"]);
    expect([crashed, yours, done, unsaid].map((row) => fromInstance(row).state.words)).toEqual(["Crashed", "Stopped by you", "Finished", "Ended"]);
  });
});
