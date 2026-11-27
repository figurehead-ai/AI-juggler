/**
 * AJ-E3820 — the turn review's words, in one place (the backend sends
 * facts and the agent's own sentences, never sentences of its own).
 */
import type { TurnReviewInfo } from "../types/generated/TurnReviewInfo";
import { THIS_MACHINE_IN_TEXT } from "./machine-words";

type Risk = "low" | "medium" | "high";

function riskOf(key: string): Risk {
  return key === "high" || key === "medium" ? key : "low";
}

export const REVIEW = {
  title: "Turn review",
  files: (n: number) => (n === 1 ? "1 file" : `${n} files`),
  noFiles: "no files",
  risk: { low: "Low risk", medium: "Medium risk", high: "High risk" } satisfies Record<Risk, string>,
  riskWord: { low: "Low", medium: "Medium", high: "High" } satisfies Record<Risk, string>,
  /** Why the risk is what it is, said by the facts' keys. */
  why: {
    failed: "it failed",
    tests_failed: "tests fail",
    failures: "something failed",
    tests_not_run: "tests not run",
    untested: "no test run mentioned",
    asks: "asks you",
    reverted: "part undone",
    large: "a large change",
  } as Record<string, string>,
  label: {
    risk: "Risk",
    changed: "Changed",
    edited: "Edited",
    reverted: "Reverted",
    tests: "Tests",
    failed: "What failed",
    asks: "Asks you",
  },
  tests: { passed: "Pass", failed: "Fail", not_run: "Not run" } as Record<string, string>,
  /** The tests come from the agent's own words, never a run of ours. */
  asItSays: "as it says",
  /** An undo that names no file: the files are "edited", not "changed". */
  unclear: "It says it undid part of this, not which file.",
  /** On the session page the reply is right above. */
  askedAbove: "its question is above",
  yes: "Yes",
  aiTag: "AI",
  aiBy: (model: string) => `Lines by ${model}`,
  /** 0.5.2 G28: while the model's lines are written (after the facts). */
  writing: (model: string | null) => `${model ?? "The model"} is writing lines on this turn…`,
  beta: "Beta",
  showWords: "Show its words",
  hideWords: "Hide its words",
  wordsLabel: "Its last reply, as the lines were checked against",
  wrong: "This is wrong",
  /** AJ-E3831 (D-154): the feed's AI summary of a turn. */
  summaryBy: (model: string) => `Summary by ${model}`,
  summaryWords: "The agent's own words",
  lastReply: "Its last reply",
  wrongKept: `Marked wrong, kept on ${THIS_MACHINE_IN_TEXT} for the next check.`,
  undo: "Undo",
  aiRisk: (risk: string) => `AI rates it ${REVIEW.riskWord[riskOf(risk)].toLowerCase()}`,
};

/** The risk's key, as one of three. */
export function riskKey(review: TurnReviewInfo): Risk {
  return riskOf(review.ai?.risk ?? review.risk);
}

/** The folded row's words: "Turn review · 2 files · Medium risk" (the
 * risk is said apart, in its colour). The files are the post's diff
 * stat's count, reverted ones too (0.5.2 G25: the post said 2 files, its
 * review "no files"). */
export function foldWords(review: TurnReviewInfo): string {
  const files = review.files;
  return `${REVIEW.title} · ${files > 0 ? REVIEW.files(files) : REVIEW.noFiles}`;
}

/** Why the risk, joined: "tests not run · part undone". */
export function whyWords(review: TurnReviewInfo, keys: readonly string[] = review.risk_why): string {
  return keys.map((key) => REVIEW.why[key]).filter(Boolean).join(" · ");
}

/** A review with something in it: a file, a revert, a failure, a
 * question or the tests. */
export function hasFindings(review: TurnReviewInfo): boolean {
  return review.changed.length > 0 || review.reverted.length > 0 || review.failures.length > 0 || review.question !== null || review.tests !== null;
}

/** The model's lines as one text (what "This is wrong" keeps). */
export function aiLines(review: TurnReviewInfo): string {
  const notes = review.changed.filter((file) => file.about).map((file) => `${file.name}: ${file.about}`);
  return [...notes, review.ai?.risk_reason ? `risk: ${review.ai.risk_reason}` : null].filter(Boolean).join("\n");
}
