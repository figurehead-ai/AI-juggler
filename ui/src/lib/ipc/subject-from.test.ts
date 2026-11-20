/** aij-test run 4: a subject made from a body is cut at a word and marked,
 * as the backend's `messaging::subject_from` does. */
import { describe, expect, it } from "vitest";
import { subjectFrom } from "./messages";

describe("a subject from a body", () => {
  it("is cut at a word near the end, with an ellipsis", () => {
    expect(subjectFrom("Using AI Juggler's pool tools, create a pool task titled 'lab task'.", 60)).toBe("Using AI Juggler's pool tools, create a pool task titled…");
  });
  it("is left as it is when short, and cut on a character when one word fills it", () => {
    expect(subjectFrom("Round tax per line", 60)).toBe("Round tax per line");
    expect(subjectFrom("x".repeat(80), 10)).toBe(`${"x".repeat(9)}…`);
  });
});
