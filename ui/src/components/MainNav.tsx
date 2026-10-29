/**
 * The header's main navigation (D-25; 0.3, the redesign §1): Home ·
 * Projects · Pools · Tasks · Accounts · Usage · History, and Fleet once a
 * Mac is paired. The lit tab is the page's title, so it comes from the
 * page shown (AJ-E1534): a project's page lights Projects, a pool's
 * Pools, an account's Accounts (lib/nav's PAGE_UNDER). The cleanup pill
 * sits with the actions on the right while there's something to tidy.
 */
import type { JSX } from "react";
import { PAGE_UNDER } from "../lib/nav";

// Snug below 1280 px, so the row fits at 1024 with the list open.
const NAV_BUTTON = "rounded-md px-1.5 py-1.5 text-meta transition-colors outline-none focus-visible:ring-2 focus-visible:ring-accent/60 xl:px-2.5 ";
// The lit tab is the page's title: raised, in ink, heavier.
const lit = (current: boolean) => (current ? "bg-surface font-semibold text-ink sheen" : "font-medium text-muted hover:bg-surface/60 hover:text-ink");

/** Whether `id`'s tab is the page shown. */
export function navLit(id: string, global: string | null, page: string): boolean {
  if (id === "home") return global === null && page === "none";
  return global === id || (global === null && PAGE_UNDER[page] === id);
}

export function MainNav<Id extends string>({
  primary,
  global,
  page,
  pairedCount,
  onPick,
}: {
  primary: Array<{ id: Id | "home"; label: string }>;
  /** The global screen shown, if any. */
  global: Id | null;
  /** The kind of item page shown, else "none". */
  page: string;
  pairedCount: number;
  onPick: (id: Id | "home") => void;
}): JSX.Element {
  const items = pairedCount > 0 ? [...primary, { id: "fleet" as Id, label: "Fleet" }] : primary;
  return (
    <nav aria-label="Main" className="flex min-w-0 flex-wrap items-center gap-0.5 xl:gap-1" data-testid="main-nav">
      {items.map((item) => {
        const current = navLit(item.id, global, page);
        return (
          <button key={item.id} type="button" aria-current={current ? "page" : undefined} onClick={() => onPick(item.id)} className={NAV_BUTTON + lit(current)} data-testid={`nav-${item.id}`}>
            {item.label}
          </button>
        );
      })}
    </nav>
  );
}

/** What's left to clean up, one click from Processes (AJ-E2404). */
export function CleanupPill({ count, shown, onOpen }: { count: number; shown: boolean; onOpen: () => void }): JSX.Element | null {
  if (count <= 0 || !shown) return null;
  return (
    <button
      type="button"
      onClick={onOpen}
      className="whitespace-nowrap rounded-full border border-warning/40 bg-warning-soft px-2.5 py-1 text-meta font-medium text-ink outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid="nav-cleanup-pill"
    >
      {count === 1 ? "1 to clean up" : `${count} to clean up`}
    </button>
  );
}
