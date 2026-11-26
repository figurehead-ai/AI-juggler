/**
 * AJ-E3820 — the preview's turn reviews, as the backend reads them from a
 * turn's own edit calls and the agent's words (generic names only): the
 * code-only reviews (`__AIJ_REVIEWS__`) and the same with the Beta's
 * lines (`__AIJ_REVIEWS_AI__`), by the turn's session or conversation id.
 */
import type { TurnReviewInfo } from "../types/generated/TurnReviewInfo";

const BASE: TurnReviewInfo = {
  state: "shown", turn: "", changed: [], reverted: [], undone_unclear: false, reverted_words: null, failures: [], question: null,
  tests: null, tests_words: null, risk: "low", risk_why: [], ai: null,
};

/** A review with the defaults the backend fills. */
export function review(over: Partial<TurnReviewInfo>): TurnReviewInfo {
  return { ...BASE, ...over };
}

/** The export streamed, its quoting reverted, untested, and a question. */
export const STREAMED: TurnReviewInfo = review({
  turn: "t-stream",
  changed: [
    { name: "export/dunnock.py", added: 40, removed: 25, about: null },
    { name: "export/writer.py", added: 12, removed: 3, about: null },
  ],
  reverted: ["export/quoting.py"],
  reverted_words: "I also changed the CSV quoting, then reverted that because two downstream readers depend on the old quoting.",
  tests: "not_run",
  tests_words: "The integration suite is still unrun.",
  question: "Should I keep the old quoting behind a flag?",
  risk: "medium",
  risk_why: ["tests_not_run", "asks", "reverted"],
});

/** The same with the Beta's lines: the first as Apple's model wrote it
 * on this turn in the real run (AJ-E3820's manifest), the second written
 * for the preview in the same style. */
export const STREAMED_AI: TurnReviewInfo = {
  ...STREAMED,
  changed: [
    { name: "export/dunnock.py", added: 40, removed: 25, about: "switched the dunnock export to stream rows" },
    { name: "export/writer.py", added: 12, removed: 3, about: "the writer now takes rows one at a time" },
  ],
  ai: {
    model: "Apple’s on-device model",
    risk: null,
    risk_reason: null,
    source:
      "Switched the dunnock export to stream rows instead of building one big list (export/dunnock.py), and the writer now takes rows one at a time. I also changed the CSV quoting, then reverted that because two downstream readers depend on the old quoting. The integration suite is still unrun.\n\nShould I keep the old quoting behind a flag?",
  },
};

/** A rewrite whose tests fail, on a turn that failed. */
export const FAILED: TurnReviewInfo = review({
  turn: "t-parser",
  changed: [{ name: "parse/godwit.ts", added: 120, removed: 96, about: null }],
  tests: "failed",
  tests_words: "Three tests still fail: test_dst_gap, test_leap_second and test_week_one.",
  failures: ["The build also hit an error in the linker step."],
  risk: "high",
  risk_why: ["failed", "tests_failed"],
});

/** A small fix, tested. */
export const CLEAN: TurnReviewInfo = review({
  turn: "t-login",
  changed: [{ name: "web/login.test.ts", added: 4, removed: 2, about: null }],
  tests: "passed",
  tests_words: "All 212 tests pass.",
  risk: "low",
});

/** A notebook (files only) and an undo naming no file. */
export const UNCLEAR: TurnReviewInfo = review({
  turn: "t-sync",
  changed: [
    { name: "sync/retry.ts", added: null, removed: null, about: null },
    { name: "notebooks/backoff.ipynb", added: null, removed: null, about: null },
  ],
  undone_unclear: true,
  reverted_words: "I also tried a longer backoff and reverted it.",
  risk: "medium",
  risk_why: ["untested", "reverted"],
});
