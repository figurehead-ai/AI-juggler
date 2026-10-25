/**
 * The header's main navigation (D-25): Home, Accounts, History, Settings;
 * Fleet once a Mac is paired; the cleanup pill when there's something to
 * tidy; More. AJ-E1534: what's lit comes from the page shown — an
 * account's page is under Accounts. (0.1.5: Usage joins the header; a
 * tool's details are in Settings › Accounts.)
 */
import type { JSX } from "react";
import { MoreMenu, type MoreItem } from "./MoreMenu";

const NAV_BUTTON = "rounded-md px-2 py-1.5 text-meta font-medium transition-colors xl:px-2.5 ";
const lit = (current: boolean) => (current ? "bg-surface text-ink sheen" : "text-muted hover:bg-surface/60 hover:text-ink");

export function MainNav<Id extends string>({
  primary,
  more,
  global,
  page,
  pairedCount,
  cleanupCount,
  onPick,
}: {
  primary: Array<{ id: Id | "home"; label: string }>;
  more: MoreItem<Id>[];
  /** The global screen shown, if any. */
  global: Id | null;
  /** The kind of item page shown, else "none". */
  page: string;
  pairedCount: number;
  cleanupCount: number;
  onPick: (id: Id | "home") => void;
}): JSX.Element {
  return (
    <nav aria-label="Main" className="flex flex-wrap items-center gap-0.5 xl:gap-1">
      {primary.map((item) => {
        const current =
          item.id === "home"
            ? global === null && page === "none"
            : global === item.id || (global === null && item.id === "accounts" && page === "account");
        return (
          <button key={item.id} type="button" aria-current={current ? "page" : undefined} onClick={() => onPick(item.id)} className={NAV_BUTTON + lit(current)}>
            {item.label}
          </button>
        );
      })}
      {pairedCount > 0 && (
        <button type="button" aria-current={global === "fleet" ? "page" : undefined} onClick={() => onPick("fleet" as Id)} className={NAV_BUTTON + lit(global === "fleet")}>
          Fleet
        </button>
      )}
      {cleanupCount > 0 && global !== "cleanup" && (
        <button
          type="button"
          onClick={() => onPick("cleanup" as Id)}
          className="ml-1 rounded-full border border-warning/40 bg-warning-soft px-2.5 py-1 text-meta font-medium text-ink"
          data-testid="nav-cleanup-pill"
        >
          {cleanupCount === 1 ? "1 thing to clean up" : `${cleanupCount} things to clean up`}
        </button>
      )}
      <MoreMenu items={more} current={global} inTools={global === null && page === "provider"} onPick={onPick} />
    </nav>
  );
}
