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
import { ROW_WORDS } from "./rows";
import { logoOf, type LogoSources } from "../../components/MemberLogo";
import { joinsLine, joinsShort } from "../../lib/pool-open-now";
import type { PreviewNode, PreviewRow, PreviewRowId } from "./NewPoolPreview";

/** The drawing's cards: who leads on top, the rest below, everyone last. */
export function previewNodes(draft: Pick<Draft, "members" | "leadNext" | "every">, nameOf: (ref: string, label: string) => string, where: string, sources: LogoSources): { lead: PreviewNode | null; members: PreviewNode[] } {
  const named = draft.members.filter((member) => member.ref !== "all");
  const short = shortNames(
    named.map((member) => nameOf(member.ref, member.label)),
    named.map((member) => member.ref),
  );
  const names = new Map(named.map((member, at) => [member.ref, short[at]!]));
  const leading = named.find((member) => member.role === "lead") ?? named.find((member) => member.ref === draft.leadNext);
  const lead: PreviewNode | null = leading
    ? { ref: leading.ref, name: names.get(leading.ref)!, line: leading.role === "lead" ? roleCard("lead").label : leadNextLine(), logo: logoOf(leading.ref, sources), role: "lead" }
    : null;
  const members: PreviewNode[] = named
    .filter((member) => member !== leading)
    .map((member) => ({ ref: member.ref, name: names.get(member.ref)!, line: roleCard(member.role).label, logo: logoOf(member.ref, sources), role: member.role }));
  const everyone = draft.members.find((member) => member.ref === "all");
  if (everyone) {
    // Three across, "Everyone / in web" (its role is its logo's mark); two
    // or fewer, "Everyone in web / Member", as the mock-ups draw it.
    const crowded = members.length >= 2 && !draft.every;
    members.push({
      ref: "all",
      name: draft.every || crowded ? "Everyone" : `Everyone in ${where}`,
      line: crowded ? `in ${where}` : roleCard(everyone.role).label,
      logo: logoOf("all", sources),
      role: everyone.role,
      stacked: true,
    });
  }
  return { lead, members };
}

const ROW_IDS: readonly PreviewRowId[] = ["where", "who", "lead", "everyone", "talk", "own"];

/** The preview's rows, one per step it's set on, each by Review's own row
 * title. `visited`: the steps walked so far (their values are chosen; the
 * rest are what they'll be, said as prefilled once Who's in is reached). */
export function previewRows(shape: PoolShape, where: string, current: SetupStep, visited: (step: SetupStep) => boolean, hasProject: boolean, openNow?: ReadonlyMap<string, number>): PreviewRow[] {
  const whoReached = visited("who") || current === "who";
  // AJ-E3014 item 4: who joins now and who later, under Who's in.
  const joins = whoReached && openNow && shape.members.length > 0 ? { short: joinsShort(shape.members, openNow), full: joinsLine(shape.members, openNow, where) } : undefined;
  const value: Record<PreviewRowId, string> = {
    where: hasProject ? where : "Pick a project",
    who: whoReached ? whoLine(shape, where) : "Next step",
    lead: leadValue(shape, where),
    everyone: rolesLine(shape),
    talk: talkSentence(shape),
    own: ownLabel(shape.level),
  };
  return ROW_IDS.map((id) => {
    const chosen = visited(id) || id === current;
    return { id, label: ROW_WORDS[id].title, value: value[id], detail: id === "who" ? joins : undefined, pending: !chosen, prefilled: !chosen && whoReached && id !== "who" && id !== "where" };
  });
}

/** The row the current step sets, if any. */
export const rowOf = (step: SetupStep): PreviewRowId | null => ROW_IDS.find((id) => id === step) ?? null;

/** A step's name in the steps (the canvas checklist's too). */
export const navOf = (step: SetupStep): string => STEP_WORDS[step].nav;
