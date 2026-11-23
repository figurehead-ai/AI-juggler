/**
 * AJ-E1554 R2 and R6: what an App's header counts, from the rows the rail
 * already holds (the cached sessions and their relation counts: instant,
 * no new scan). Each count keeps the items it counts, so a click on it
 * opens exactly those, by id, and the number can't disagree with them.
 */
import type { RailItem } from "./rail-items";

export type CountKind = "working" | "needs" | "review" | "subagents";

export interface HeadCount {
  kind: CountKind;
  /** The items it counts (for subagents, the sessions that run some). */
  items: RailItem[];
  /** The number shown (for subagents, how many run). */
  n: number;
}

/** An App's counts, non-zero only, in the header's order: working, needs
 * you (what Needs you counts here), done to review, subagents running. */
export function headCounts(items: readonly RailItem[], needItems: readonly RailItem[] = []): HeadCount[] {
  const needs = new Set(needItems.map((item) => item.key));
  const working = items.filter((item) => item.state === "working" && !needs.has(item.key));
  const review = items.filter((item) => item.state === "done_review" && !needs.has(item.key));
  const agents = items.filter((item) => item.subagents > 0);
  const all: HeadCount[] = [
    { kind: "working", items: working, n: working.length },
    { kind: "needs", items: [...needItems], n: needItems.length },
    { kind: "review", items: review, n: review.length },
    { kind: "subagents", items: agents, n: agents.reduce((sum, item) => sum + item.subagents, 0) },
  ];
  return all.filter((count) => count.n > 0);
}

/** A rail item's id, as the rail selects it. */
export function idOf(item: RailItem): string | null {
  return item.session?.id ?? item.conversation?.id ?? null;
}

/** The one a count's click opens: the next of its items after the one open
 * now, the first when none of them is (each click, the next). */
export function nextOf(items: readonly RailItem[], selectedId: string | null): RailItem | null {
  if (items.length === 0) return null;
  const at = items.findIndex((item) => idOf(item) === selectedId);
  return items[(at + 1) % items.length] ?? null;
}

/** What a count says, for its tooltip and its name. */
export function countWords(count: HeadCount): string {
  const { n, kind, items } = count;
  const each = items.length === 1 ? "opens it" : "each click opens the next";
  const words: Record<CountKind, string> = {
    working: `${n} working here: ${each}`,
    needs: `${n} here ${n === 1 ? "needs" : "need"} you: ${each}`,
    review: `${n} done, to review: ${each}`,
    subagents: `${n} ${n === 1 ? "subagent" : "subagents"} running${items.length > 1 ? ` in ${items.length} sessions` : ""}: opens ${items.length === 1 ? "its" : "each one's"} Subagents tab`,
  };
  return words[kind];
}
