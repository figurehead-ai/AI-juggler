/**
 * D-62 (AJ-E3202 req. 4) — an Account's plan, renewal and fee, in words.
 * They're set once on the Account; where its Apps said different things
 * before 0.4, the Account asks once: "Work and Work 2 say different
 * renewal dates: 12 Nov or 3 Dec?", with what the choice changes.
 */
import { planWords } from "./subscriptions";
import { dateWords } from "./time-words";
import type { SubscriptionInfo, Term, TermChoiceInfo } from "./ipc/subscriptions";

/** What a term is called, one and several. */
const NOUN: Record<Term, [string, string]> = {
  plan: ["plan", "plans"],
  renewal: ["renewal date", "renewal dates"],
  fee: ["monthly fee", "monthly fees"],
};

/** What choosing changes, in plain words. */
const CONSEQUENCE: Record<Term, string> = {
  plan: "An account has one plan: it's shown wherever the account is named, for every app on it.",
  renewal: "An account has one renewal date: renewal reminders and its card use it, for every app on it.",
  fee: "An account has one monthly fee: the cost view weighs your usage against it, for every app on it.",
};

export function isTerm(raw: string): raw is Term {
  return raw === "plan" || raw === "renewal" || raw === "fee";
}

/** "plan", "renewal date", "monthly fee" (`many`: "renewal dates"). */
export function termNoun(term: Term, many = false): string {
  return NOUN[term][many ? 1 : 0];
}

/** What choosing `term` changes, and that nothing changes until then. */
export function termConsequence(term: Term): string {
  return `${CONSEQUENCE[term]} Until you choose, each app keeps its own.`;
}

/** A value as it's said: "Max 20x", "12 Nov" ("12 Nov 2027" in another
 * year than `now`'s), "$200/month" (as the Accounts cards say it). */
export function termValueWords(term: Term, value: string, now: number): string {
  if (term === "plan") return planWords(value);
  if (term === "fee") return `$${value}/month`;
  return dateWords(value, now);
}

/** "A", "A and B", "A, B and C". */
export function listWords(names: readonly string[]): string {
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** Who says option `option`: its folders' names, or "This account" for
 * the value the Account already has. */
export function whoSays(sub: SubscriptionInfo, option: TermChoiceInfo["options"][number]): string[] {
  const names = option.folders.map((id) => sub.profiles.find((profile) => profile.id === id)?.label ?? "an app");
  return option.account ? ["This account", ...names] : names;
}

/** The question: "Work and Work 2 say different renewal dates: 12 Nov or
 * 3 Dec?" */
export function choiceQuestion(sub: SubscriptionInfo, choice: TermChoiceInfo, now: number): string {
  const term = isTerm(choice.term) ? choice.term : "plan";
  const who = choice.options.flatMap((option) => whoSays(sub, option));
  const values = choice.options.map((option) => termValueWords(term, option.value, now));
  const or = values.length <= 1 ? (values[0] ?? "") : `${values.slice(0, -1).join(", ")} or ${values[values.length - 1]}`;
  return `${listWords(who)} say different ${termNoun(term, true)}: ${or}?`;
}

/** What an Account asks you to choose, as a call to act: "Choose its
 * renewal date", "Choose its plan and monthly fee". */
export function chooseWords(sub: SubscriptionInfo): string {
  return `Choose its ${listWords(sub.choices.map((choice) => (isTerm(choice.term) ? termNoun(choice.term) : choice.term)))}`;
}
