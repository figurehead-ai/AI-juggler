/**
 * 0.4.6 (D-126): each setup step's value in a few words, under its name
 * — New pool's steps and the pool canvas's checklist — and New pool's
 * preview rows, all read from one {@link PoolShape} (a draft's through
 * `shapeOfDraft`, a made pool's through `shapeOfPool`). One function per
 * value, so the wizard and the diagram never say one pool two ways.
 *
 * A member goes by the name its card has, short where nothing else
 * shares it ({@link shortNames}); past three names, the rest are counted.
 */
import { listWords } from "./account-terms";
import type { TalkRule } from "./ipc/pool-model";
import type { PoolShape, ShapeMember } from "./pool-sentences";
import type { PairKey } from "./pool-templates";
import { NO_LEAD_YET, OWN_MIX, TALK_PRESETS, nextSessionWords, ownLabel, presetOf, type SETUP_STEPS } from "./pool-words";

type Step = (typeof SETUP_STEPS)[number];

/** At most this many names in a value; then "2 more". */
const NAMES = 3;

/** Each member by one short identity (D-126: values fit their place):
 * an App by its app word where no other member shares it ("Claude",
 * "Codex" for "Codex · Personal"); Apps that share one by their profiles
 * ("Work Max", "Personal" for two "Claude Code CLI · …"); anything else
 * that shares a name, whole. `refs`, beside the labels, says which are
 * Apps; without them every label is taken for one. */
export function shortNames(labels: readonly string[], refs?: readonly string[]): string[] {
  const head = (label: string) => label.split(" · ")[0]!;
  const tail = (label: string) => {
    const at = label.indexOf(" · ");
    return at < 0 ? null : label.slice(at + 3);
  };
  return labels.map((label, at) => {
    const shared = labels.filter((one) => head(one) === head(label));
    if (shared.length === 1) return head(label);
    const app = refs ? (refs[at] ?? "").startsWith("app:") : true;
    const profile = tail(label);
    return app && profile !== null && shared.filter((one) => tail(one) === profile).length === 1 ? profile : label;
  });
}

/** Three names, then how many more: ["A", "B", "C", "2 more"]. */
const fewOf = (names: readonly string[]): string[] => (names.length > NAMES ? [...names.slice(0, NAMES), `${names.length - NAMES} more`] : [...names]);

/** The members by their short names (everyone in the projects included,
 * by its own name). */
function named(shape: Pick<PoolShape, "members">): (ShapeMember & { name: string })[] {
  const names = shortNames(
    shape.members.map((member) => member.label),
    shape.members.map((member) => member.ref),
  );
  return shape.members.map((member, at) => ({ ...member, name: names[at]! }));
}

const someone = (member: { ref: string }) => member.ref !== "all";
const everyoneIn = (shape: Pick<PoolShape, "members">) => shape.members.some((member) => member.ref === "all");

/** Who's in: "Claude, Codex, everyone in web"; "Everyone in web"; "No
 * one yet". */
export function whoValue(shape: Pick<PoolShape, "members">, where: string): string {
  const names = fewOf(named(shape).filter(someone).map((member) => member.name));
  if (!everyoneIn(shape)) return names.length === 0 ? "No one yet" : names.join(", ");
  return names.length === 0 ? `Everyone in ${where}` : [...names, `everyone in ${where}`].join(", ");
}

/** Who leads: the session that does, the App whose next session will
 * ("Claude's next session", with `where`: "… in web"), or none yet. */
export function leadValue(shape: Pick<PoolShape, "members" | "leadNext">, where?: string): string {
  const members = named(shape);
  const session = members.find((member) => member.role === "lead" && member.ref.startsWith("session:"));
  if (session) return session.name;
  const next = shape.leadNext ? members.find((member) => member.ref === shape.leadNext) : undefined;
  if (!next) return NO_LEAD_YET;
  return where ? `${nextSessionWords(next.name)} in ${where}` : nextSessionWords(next.name);
}

/** Everyone builds, unless someone checks. */
const EVERYONE_BUILDS = "Everyone builds";

/** Roles: "Codex checks the work", else "Everyone builds". Everyone in
 * the projects joining as a Reviewer checks the work too. */
export function rolesValue(shape: Pick<PoolShape, "members">): string {
  const reviewers = named(shape)
    .filter((member) => member.role === "reviewer")
    .map((member, at) => (at > 0 && !someone(member) ? member.name.replace(/^Everyone/, "everyone") : member.name));
  if (reviewers.length === 0) return EVERYONE_BUILDS;
  return `${listWords(fewOf(reviewers))} ${reviewers.length === 1 ? "checks" : "check"} the work`;
}

/** Who may message whom: the way's own words, or its own mix. */
export function talkValue(talk: Partial<Record<PairKey, TalkRule>>): string {
  const preset = presetOf(talk);
  return TALK_PRESETS.find((one) => one.id === preset)?.short ?? OWN_MIX;
}

/** Each of the setup's steps' value under its name: the same words in
 * New pool's steps and on the canvas's checklist. */
export function stepValues(shape: Pick<PoolShape, "members" | "leadNext" | "talk" | "level">, where: string): Record<Step, string> {
  return {
    where,
    who: whoValue(shape, where),
    lead: leadValue(shape),
    everyone: rolesValue(shape),
    talk: talkValue(shape.talk),
    // The level's name, as the header's picker says it ("Custom" too).
    own: ownLabel(shape.level),
  };
}

/* ---------------- New pool's preview: its rows, a line each ---------------- */

/** Who's in as a line: "Claude and Codex, and every new session in web". */
export function whoLine(shape: Pick<PoolShape, "members">, where: string): string {
  const names = listWords(fewOf(named(shape).filter(someone).map((member) => member.name)));
  if (!everyoneIn(shape)) return names || "No one yet";
  return names ? `${names}, and every new session in ${where}` : `Every new session in ${where}`;
}

/** Roles as a line: "Codex checks the work; the rest build", else
 * "Everyone builds (Member)". */
export function rolesLine(shape: Pick<PoolShape, "members">): string {
  const value = rolesValue(shape);
  return value === EVERYONE_BUILDS ? `${EVERYONE_BUILDS} (Member)` : `${value}; the rest build`;
}

/* ---------------- Review: a short sentence a row ---------------- */

/** What they may do on their own, short: "They work together". */
const OWN_SHORT: Record<string, string> = { read: "They look only", work_together: "They work together", full: "They work together and wake each other", custom: "They do the things you picked" };

/** Review's rows, one short sentence each, from the same values as the
 * steps (D-126, the mock-ups': "Codex checks the work. Everyone else
 * builds."); every rule in full is the canvas's own sentences, one click
 * away under them. */
export function reviewLines(shape: Pick<PoolShape, "members" | "leadNext" | "talk" | "level" | "sealed">, where: string, name: string): Record<"where" | "who" | "lead" | "everyone" | "talk" | "own" | "sealed", string> {
  const lead = leadValue(shape, where);
  const roles = rolesValue(shape);
  const preset = presetOf(shape.talk);
  const way = TALK_PRESETS.find((one) => one.id === preset);
  const talk = preset === "free" ? "Everyone may message everyone." : (way?.line ?? `${OWN_MIX}: some pairs ask you first or are blocked.`);
  return {
    where: `“${name}” works in ${where}.`,
    who: `${whoLine(shape, where)}.`,
    lead: lead === NO_LEAD_YET ? `${NO_LEAD_YET}: each session works on what you give it.` : `${lead} leads.`,
    everyone: roles === EVERYONE_BUILDS ? `${EVERYONE_BUILDS}.` : `${roles}. Everyone else builds.`,
    talk,
    own: `${OWN_SHORT[shape.level] ?? OWN_SHORT.work_together}. Anything more asks you.`,
    sealed: shape.sealed ? "Yes. Nothing goes in or out, and its sessions are in no other pool." : "No. It can link to your other pools.",
  };
}
