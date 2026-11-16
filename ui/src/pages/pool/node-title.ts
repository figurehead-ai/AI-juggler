/**
 * 0.4.6 (D-126 §4a, the rc1 X2 lab: fifty nodes all read "web · started
 * 5:16 P…"): what a session's node leads with, so fifty members are told
 * apart at a glance —
 *
 * - its own name (its label, its title's words or a branch that says what
 *   the work was: `shortName`);
 * - else the task it holds;
 * - else its folder ("Session" when it has none);
 *
 * told apart by a number where two still read alike ("web (12)"), and
 * when it started second, on a line of its own, never in front of the
 * name. A session with a name of its own that no other member shares
 * needs no time.
 */
import { folderName, namedByFacts, shortName, startedWords, tellApart } from "../../lib/session-titles";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

/** The card's word for no task (cards.rs's NO_TASK). */
const NO_TASK = "None";

export type NodeTitle = { title: string; when: string | null };

type Titled = { id: string; session: Pick<InstanceInfo, "label" | "title" | "git_branch" | "cwd" | "started_at_ms">; card: { task: { value: string | null } } };

/** Each member's title and its start, by id. */
export function nodeTitles(members: readonly Titled[], now: number = Date.now()): Map<string, NodeTitle> {
  const rows = members.map((member) => {
    const own = namedByFacts(member.session) ? null : shortName(member.session, now);
    const task = member.card.task.value && member.card.task.value !== NO_TASK ? member.card.task.value : null;
    return { member, own, base: own ?? task ?? (folderName(member.session.cwd) || "Session") };
  });
  const apart = tellApart(rows, (row) => row.base, (row) => [row.member.session.started_at_ms ?? 0, row.member.id]);
  const out = new Map<string, NodeTitle>();
  for (const row of rows) {
    const title = apart.get(row) ?? row.base;
    const started = row.member.session.started_at_ms;
    const when = row.own !== null && title === row.base ? null : started && started > 0 ? startedWords(started, now) : null;
    out.set(row.member.id, { title, when });
  }
  return out;
}

/** The words a node's button is named by: its title, then when it
 * started (the visible title first, WCAG 2.5.3). */
export const titleWords = (title: NodeTitle): string => (title.when ? `${title.title}, ${title.when}` : title.title);
