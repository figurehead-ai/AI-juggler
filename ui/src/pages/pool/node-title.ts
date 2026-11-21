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
import { folderName, namedByFacts, numberAlike, shortName, startedWords } from "../../lib/session-titles";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

/** The card's word for no task (cards.rs's NO_TASK). */
const NO_TASK = "None";

export type NodeTitle = { title: string; when: string | null };

type Titled = { id: string; session: Pick<InstanceInfo, "label" | "title" | "git_branch" | "cwd" | "started_at_ms">; card: { task: { value: string | null } } };

/** The number a session's one name tells it apart by ("web · started
 * 8:33 AM (1)": 1), when it has one. */
function numberIn(name: string | undefined): string | null {
  return name?.match(/ \((\d+)\)$/)?.[1] ?? null;
}

/** Each member's title and its start, by id. `names`: each session's one
 * name (`sessionNames`, SK48-3): a session named by its facts keeps the
 * number it has everywhere ("web (1)" in every pool, as the rail's "web
 * · started 8:33 AM (1)"), never one of this pool's own. */
export function nodeTitles(members: readonly Titled[], now: number = Date.now(), names: ReadonlyMap<string, string> = new Map()): Map<string, NodeTitle> {
  const rows = members.map((member) => {
    const own = namedByFacts(member.session) ? null : shortName(member.session, now);
    const task = member.card.task.value && member.card.task.value !== NO_TASK ? member.card.task.value : null;
    const folder = folderName(member.session.cwd) || "Session";
    const number = own === null && task === null ? numberIn(names.get(member.id)) : null;
    return { member, own, base: own ?? task ?? (number ? `${folder} (${number})` : folder) };
  });
  // One named by its facts shows when it started under its title: only
  // what reads alike with that too is numbered here (SK48-3: two "web"
  // started at 8:33 and 9:10 aren't "web (1)" and "web (2)" in one pool
  // and "web" in another).
  const whenOf = (row: (typeof rows)[number]) => {
    const at = row.member.session.started_at_ms;
    return at && at > 0 ? startedWords(at, now) : null;
  };
  const key = (row: (typeof rows)[number]) => (row.own === null ? `${row.base}\n${whenOf(row) ?? ""}` : row.base);
  const numbers = numberAlike(rows, key, (row) => [row.member.session.started_at_ms ?? 0, row.member.id]);
  const apart = new Map(rows.map((row) => [row, numbers.get(row) ? `${row.base} (${numbers.get(row)})` : row.base] as const));
  const out = new Map<string, NodeTitle>();
  for (const row of rows) {
    const title = apart.get(row) ?? row.base;
    const when = row.own !== null && title === row.base ? null : whenOf(row);
    out.set(row.member.id, { title, when });
  }
  return out;
}

/** The words a node's button is named by: its title, then when it
 * started (the visible title first, WCAG 2.5.3). */
export const titleWords = (title: NodeTitle): string => (title.when ? `${title.title}, ${title.when}` : title.title);
