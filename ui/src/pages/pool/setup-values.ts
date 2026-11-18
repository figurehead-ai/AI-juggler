/**
 * 0.4.6 (D-126 §3): each checklist row's value, in a few words under its
 * name ("Who leads · Claude's next session"), read from the same shape as
 * the pool's sentences (PoolShape), so the checklist, the summary and New
 * pool's Review never say one pool two ways.
 */
import { listWords } from "../../lib/account-terms";
import type { PoolShape } from "../../lib/pool-sentences";
import { effectiveRule, ruleOf, TALK_ROLES } from "../../lib/pool-templates";
import { nextSessionWords, ownMay, type SETUP_STEPS } from "../../lib/pool-words";

type Step = (typeof SETUP_STEPS)[number];

/** At most this many names in a row's value; then "and N more". */
const NAMES = 3;

/** "Claude, Codex, everyone in web"; past three, "and N more". */
function whoValue(shape: PoolShape, where: string): string {
  const named = shape.members.filter((member) => member.ref !== "all").map((member) => member.label);
  const everyone = shape.members.some((member) => member.ref === "all");
  if (named.length === 0) return everyone ? `Everyone in ${where}` : "No one yet";
  const shown = named.length > NAMES ? [...named.slice(0, NAMES), `${named.length - NAMES} more`] : named;
  return everyone ? `${shown.join(", ")}, everyone in ${where}` : listWords(shown);
}

/** The session that leads, the App whose next session will, or none. */
function leadValue(shape: PoolShape): string {
  const lead = shape.members.find((member) => member.role === "lead" && member.ref.startsWith("session:"));
  if (lead) return lead.label;
  const next = shape.leadNext ? shape.members.find((member) => member.ref === shape.leadNext) : undefined;
  return next ? nextSessionWords(next.label) : "No lead for now";
}

/** Who checks the work, else that everyone builds. */
function rolesValue(shape: PoolShape): string {
  const reviewers = shape.members.filter((member) => member.role === "reviewer").map((member) => member.label);
  if (reviewers.length === 0) return "Everyone builds";
  const shown = reviewers.length > NAMES ? [...reviewers.slice(0, NAMES), `${reviewers.length - NAMES} more`] : reviewers;
  return `${listWords(shown)} ${reviewers.length === 1 ? "checks" : "check"} the work`;
}

/** Everyone freely, or the exceptions in a word. An Observer only reads
 * (its messages always wait for you), so it's no exception. */
function talkValue(shape: PoolShape): string {
  const senders = TALK_ROLES.filter((from) => from !== "observer");
  const rules = senders.flatMap((from) => TALK_ROLES.map((to) => effectiveRule(from, ruleOf(shape.talk, from, to))));
  if (rules.includes("blocked")) return "Some may not message";
  if (rules.includes("ask")) return "Some ask you first";
  return "Everyone, freely";
}

export function checklistValues(shape: PoolShape, where: string): Record<Step, string> {
  return {
    where,
    who: whoValue(shape, where),
    lead: leadValue(shape),
    everyone: rolesValue(shape),
    talk: talkValue(shape),
    own: ownMay(shape.level, shape.picked),
  };
}
