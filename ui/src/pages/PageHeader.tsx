/**
 * AJ-E1533 — an item's identity header on its page (PageShell): its
 * mark and name, its parent as the crumb, its state and provenance chips,
 * two or three facts, and its actions with its ⋯ menu.
 */
import { useLayoutEffect, useRef, type JSX, type ReactNode, type RefObject } from "react";
import { useKeys } from "../lib/use-keys";
import { PAGE_LABELS, type PageKind } from "./registry";

/** Marks `row` (`data-actions-below`) while its last child, the actions,
 * sits a line below its first, the names (D-100: they went down rather
 * than cut a name). Their hints take no height beside the names; below
 * them they get room, or they print over the facts line. */
function useActionsBelow(row: RefObject<HTMLElement | null>, on: boolean): void {
  useLayoutEffect(() => {
    const box = row.current;
    const names = box?.firstElementChild;
    const actions = box?.lastElementChild;
    if (!on || !box || !(names instanceof HTMLElement) || !(actions instanceof HTMLElement) || names === actions) return undefined;
    const check = () => box.toggleAttribute("data-actions-below", actions.offsetTop > names.offsetTop);
    check();
    if (typeof ResizeObserver === "undefined") return undefined;
    // A line more or less changes its height: so does a width that moves them.
    const watch = new ResizeObserver(check);
    watch.observe(box);
    return () => watch.disconnect();
  }, [row, on]);
}

/** A link to the page's parent (its project, its tool, Accounts). */
export type Crumb = { label: string; onOpen?: () => void; testId?: string };

/** A breadcrumb's link, in the line's own size: a page's header and a
 * pool's share it. Under 24 px tall, so the header keeps it clear of the
 * targets under it (WCAG 2.5.8's spacing). */
export function CrumbButton({ onClick, testId, children }: { onClick: () => void; testId?: string; children: ReactNode }): JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      data-testid={testId}
      className="truncate rounded outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      {children}
    </button>
  );
}

export function PageHeader({
  kind,
  mark,
  name,
  crumbs = [],
  chips,
  facts,
  actions,
  folded = false,
  onToggleFold,
  layout = "item",
  here,
}: {
  kind: PageKind;
  mark: ReactNode;
  name: string;
  crumbs?: Crumb[];
  /** State and provenance chips beside the name. */
  chips?: ReactNode;
  /** Two or three facts of this type, one line under the name. */
  facts?: ReactNode;
  /** Primary buttons, then the item's ⋯ menu. */
  actions?: ReactNode;
  folded?: boolean;
  onToggleFold?: () => void;
  /** AJ-E2108: a session's page: a large mark, a larger name, and its
   * chips on their own line under it. */
  layout?: "item" | "session";
  /** 0.3: the breadcrumb's last step, when not the type's word
   * ("Projects › storefront"). */
  here?: string;
}) {
  const inspectorKeys = useKeys("inspector");
  const session = layout === "session";
  const namesRow = useRef<HTMLDivElement | null>(null);
  useActionsBelow(namesRow, session);
  const title = (
    <h2
      tabIndex={-1}
      className={"min-w-0 truncate tracking-tight outline-none " + (session ? "text-title font-semibold leading-tight" : "text-lg font-semibold")}
      // A long name ends in an ellipsis; all of it on hover.
      title={name}
      data-testid="page-name"
    >
      {name}
    </h2>
  );
  const actionsBox = (
    <div className="ml-auto flex shrink-0 items-center gap-2 group-data-[actions-below]/names:has-data-hint:pb-9">
      {actions}
      {onToggleFold && (
        <button
          type="button"
          onClick={onToggleFold}
          aria-expanded={!folded}
          aria-label={folded ? "Unfold the inspector" : "Fold the inspector"}
          title={folded ? `Unfold the inspector (${inspectorKeys.words})` : `Fold the inspector (${inspectorKeys.words})`}
          data-testid="inspector-fold"
          className="rounded-lg p-1 text-muted transition-colors hover:bg-surface-2 hover:text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d={folded ? "M6 9l6 6 6-6" : "M6 15l6-6 6 6"} />
          </svg>
        </button>
      )}
    </div>
  );
  return (
    <header className="flex flex-col gap-1" data-testid="page-header" data-page-kind={kind}>
      <nav aria-label="Breadcrumb" className="flex min-w-0 items-center gap-1 text-meta text-muted" data-testid="page-crumbs">
        {crumbs.map((crumb) => (
          <span key={crumb.label} className="flex min-w-0 items-center gap-1">
            {crumb.onOpen ? (
              <CrumbButton onClick={crumb.onOpen} testId={crumb.testId}>
                {crumb.label}
              </CrumbButton>
            ) : (
              <span className="truncate">{crumb.label}</span>
            )}
            <span aria-hidden="true">›</span>
          </span>
        ))}
        <span className="font-medium" data-testid="page-type">
          {here ?? PAGE_LABELS[kind]}
        </span>
      </nav>
      <div className={session ? "flex items-start gap-4 pt-1" : "flex flex-wrap items-center gap-3"}>
        <span className={session ? "shrink-0" : "text-accent-hi"}>{mark}</span>
        {session ? (
          // D-100 (0.4.4): names give way last. Where the name and its
          // chips can't keep their names beside the actions, the actions go
          // down a line: the chips' room is what they need with all their
          // secondary words left out (`--give-way-need`, use-give-way),
          // folded with the title's 8rem beside them. Their App chip read
          // nothing at 1024 px beside an ended session's four actions.
          <div ref={namesRow} className="group/names flex min-w-0 flex-1 flex-wrap items-start gap-x-4 gap-y-2">
            <div
              data-give-way-room=""
              className={"@container flex min-w-0 grow flex-col gap-2 " + (folded && chips ? "basis-[calc(8.75rem_+_var(--give-way-need,0px))]" : "basis-[var(--give-way-need,0px)]")}
            >
              {folded && chips ? (
                // rc22 (walk A3): folded, the chips stay, on the name's line
                // (its App chip is one click to its page; it was hidden
                // until the header was unfolded). 0.4.1: the title truncates first (to 8rem), then the chips.
                <div className="flex min-w-0 flex-nowrap items-center gap-3">
                  {title}
                  <div className="flex min-w-0 max-w-[calc(100%-8rem)] shrink-0 flex-nowrap items-center gap-2" data-testid="page-chips">
                    {chips}
                  </div>
                </div>
              ) : (
                title
              )}
              {!folded && chips && (
                // One line (the review of 0.1.5: two rows at 1280): chips
                // shrink to an ellipsis, and the less telling ones fold
                // behind "…" where it's narrow.
                <div className="flex min-w-0 flex-nowrap items-center gap-2" data-testid="page-chips">
                  {chips}
                </div>
              )}
            </div>
            {actionsBox}
          </div>
        ) : (
          <>
            {title}
            {chips}
            {actionsBox}
          </>
        )}
      </div>
      {!folded && facts && (
        <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-meta text-muted" data-testid="page-facts">
          {facts}
        </div>
      )}
    </header>
  );
}
