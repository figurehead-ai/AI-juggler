/**
 * 0.4.6 (D-126): what New pool's preview shows, from the draft — one
 * place, so the drawing and its rows can't drift from the form: the rows
 * read the draft's shape through the shared values (pool-values, the
 * steps' and the canvas checklist's words), the talk row is Review's own
 * sentence.
 */
import type { Draft } from "../../lib/new-pool";
import { talkSentence, type PoolShape } from "../../lib/pool-sentences";
import { leadValue, rolesLine, shortNames, whoLine } from "../../lib/pool-values";
import { leadNextLine, ownLabel, roleCard, STEP_WORDS, type SetupStep } from "../../lib/pool-words";
import { logoOf, type LogoSources } from "../../components/MemberLogo";
import type { PreviewNode, PreviewRow, PreviewRowId } from "./NewPoolPreview";

/** The drawing's cards: who leads on top, the rest below, everyone last. */
export function previewNodes(draft: Pick<Draft, "members" | "leadNext" | "every">, nameOf: (ref: string, label: string) => string, where: string, sources: LogoSources): { lead: PreviewNode | null; members: PreviewNode[] } {
  const named = draft.members.filter((member) => member.ref !== "all");
  const short = shortNames(named.map((member) => nameOf(member.ref, member.label)));
  const names = new Map(named.map((member, at) => [member.ref, short[at]!]));
  const leading = named.find((member) => member.role === "lead") ?? named.find((member) => member.ref === draft.leadNext);
  const lead: PreviewNode | null = leading
    ? { ref: leading.ref, name: names.get(leading.ref)!, line: leading.role === "lead" ? roleCard("lead").label : leadNextLine(), logo: logoOf(leading.ref, sources), role: "lead" }
    : null;
  const members: PreviewNode[] = named
    .filter((member) => member !== leading)
    .map((member) => ({ ref: member.ref, name: names.get(member.ref)!, line: roleCard(member.role).label, logo: logoOf(member.ref, sources), role: member.role }));
  const everyone = draft.members.find((member) => member.ref === "all");
  if (everyone) members.push({ ref: "all", name: draft.every ? "Everyone" : `Everyone in ${where}`, line: roleCard(everyone.role).label, logo: logoOf("all", sources), role: everyone.role, stacked: true });
  return { lead, members };
}

/** The step each preview row is set on. */
export const ROW_STEP: Record<PreviewRowId, SetupStep> = { where: "where", who: "who", lead: "lead", roles: "everyone", talk: "talk", own: "own" };

const ROW_IDS: readonly PreviewRowId[] = ["where", "who", "lead", "roles", "talk", "own"];

/** The preview's rows, each by its step's short name (the canvas
 * checklist's). `visited`: the steps walked so far (their values are
 * chosen; the rest are what they'll be, said as prefilled once Who's in
 * is reached). */
export function previewRows(shape: PoolShape, where: string, current: SetupStep, visited: (step: SetupStep) => boolean, hasProject: boolean): PreviewRow[] {
  const whoReached = visited("who") || current === "who";
  const value: Record<PreviewRowId, string> = {
    where: hasProject ? where : "Pick a project",
    who: whoReached ? whoLine(shape, where) : "Next step",
    lead: leadValue(shape, where),
    roles: rolesLine(shape),
    talk: talkSentence(shape),
    own: ownLabel(shape.level),
  };
  return ROW_IDS.map((id) => {
    const step = ROW_STEP[id];
    const chosen = visited(step) || step === current;
    return { id, label: STEP_WORDS[step].short, value: value[id], pending: !chosen, prefilled: !chosen && whoReached && id !== "who" && id !== "where" };
  });
}

/** The row the current step sets, if any. */
export const rowOf = (step: SetupStep): PreviewRowId | null => ROW_IDS.find((id) => ROW_STEP[id] === step) ?? null;

/** A step's name in the steps (the canvas checklist's too). */
export const navOf = (step: SetupStep): string => STEP_WORDS[step].nav;
