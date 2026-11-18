/**
 * D-118's mock (Home round 3) — one order for what needs you, in the rail
 * and on Home: the louder level first (what broke, L3, over what waits,
 * L2), then the one waiting longest. The rail listed a session's need
 * before an agent's question while Home listed the question first; both
 * now merge their lists by this rule.
 *
 * Each list keeps its own order (a session's need by its kind, then its
 * wait, `rail-items`' `byNeed`; Home's kinds): the lists are merged, never
 * re-sorted, so nothing a list already decided changes.
 */
import { itemLevel, noticeLevel } from "./attention";
import { waitingSince } from "./waiting";
import type { NoticeInfo } from "./ipc/notices";
import type { RailItem } from "./rail-items";
import type { Line } from "./rail-lines";

/** Where a need stands in the order: its attention level and when it
 * began to wait (epoch ms). */
export interface Waits {
  level: number;
  since: number;
}

/** Whether `a` goes strictly before `b`: the louder first, then the older. */
function before(a: Waits, b: Waits): boolean {
  return a.level !== b.level ? a.level > b.level : a.since < b.since;
}

/** The lists merged into one, by `waits`: at each step the head that goes
 * first (a tie to the earlier list). */
export function mergeByWait<T>(lists: readonly (readonly T[])[], waits: (item: T) => Waits): T[] {
  const at = lists.map(() => 0);
  const out: T[] = [];
  for (;;) {
    let pick: { list: number; item: T } | null = null;
    lists.forEach((list, index) => {
      const head = list[at[index]!];
      if (head !== undefined && (pick === null || before(waits(head), waits(pick.item)))) pick = { list: index, item: head };
    });
    if (pick === null) return out;
    const { list, item } = pick as { list: number; item: T };
    out.push(item);
    at[list] = at[list]! + 1;
  }
}

/** The rail's Needs you lines that count, sessions' and the rest's (an
 * agent's question, a sign-in…) merged by this rule; a notice waits since
 * it was first written. */
export function loudLines(needs: RailItem[], rest: NoticeInfo[]): Line[] {
  const lines: Line[][] = [needs.map((item): Line => ({ kind: "need", item })), rest.map((notice): Line => ({ kind: "notice", notice }))];
  return mergeByWait(lines, (line) =>
    line.kind === "need" ? { level: itemLevel(line.item), since: waitingSince(line.item) } : line.kind === "notice" ? { level: noticeLevel(line.notice), since: line.notice.created_ms } : { level: 0, since: 0 },
  );
}
