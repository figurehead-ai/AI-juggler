/**
 * AJ-E1533 — the one page every item opens in (pages/registry.ts): an
 * identity header, then the page's tabs, the first its body. Panels
 * toggle `hidden` (a terminal never remounts on a tab switch); the tab
 * a type of page was left on is remembered for that type only.
 */
import { useEffect, useRef, useState, type ReactNode } from "react";
import { matches } from "../lib/shortcuts";
import { statsEvent } from "../lib/bridge";
import { PAGE_LABELS, rememberTab, rememberedTab, type PageKind, type PageTab, type PageTabId } from "./registry";

/** A link to the page's parent (its project, its tool, Accounts). */
export type Crumb = { label: string; onOpen?: () => void; testId?: string };

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
}) {
  const session = layout === "session";
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
  return (
    <header className="flex flex-col gap-1" data-testid="page-header" data-page-kind={kind}>
      <nav aria-label="Breadcrumb" className="flex min-w-0 items-center gap-1 text-meta text-muted" data-testid="page-crumbs">
        {crumbs.map((crumb) => (
          <span key={crumb.label} className="flex min-w-0 items-center gap-1">
            {crumb.onOpen ? (
              <button type="button" onClick={crumb.onOpen} data-testid={crumb.testId} className="truncate rounded outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
                {crumb.label}
              </button>
            ) : (
              <span className="truncate">{crumb.label}</span>
            )}
            <span aria-hidden="true">›</span>
          </span>
        ))}
        <span className="font-medium" data-testid="page-type">
          {PAGE_LABELS[kind]}
        </span>
      </nav>
      <div className={session ? "flex items-start gap-4 pt-1" : "flex flex-wrap items-center gap-3"}>
        <span className={session ? "shrink-0" : "text-accent-hi"}>{mark}</span>
        {session ? (
          <div className="@container flex min-w-0 flex-1 flex-col gap-2">
            {title}
            {!folded && chips && (
              // One line (the review of 0.1.5: two rows at 1280): chips
              // shrink to an ellipsis, and the less telling ones fold
              // behind "…" where it's narrow.
              <div className="flex min-w-0 flex-nowrap items-center gap-2" data-testid="page-chips">
                {chips}
              </div>
            )}
          </div>
        ) : (
          <>
            {title}
            {chips}
          </>
        )}
        <div className="ml-auto flex shrink-0 items-center gap-2">
          {actions}
          {onToggleFold && (
            <button
              type="button"
              onClick={onToggleFold}
              aria-expanded={!folded}
              aria-label={folded ? "Unfold the inspector" : "Fold the inspector"}
              title={folded ? "Unfold the inspector (⌘⇧I)" : "Fold the inspector (⌘⇧I)"}
              data-testid="inspector-fold"
              className="rounded-lg p-1 text-muted transition-colors hover:bg-surface-2 hover:text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d={folded ? "M6 9l6 6 6-6" : "M6 15l6-6 6 6"} />
              </svg>
            </button>
          )}
        </div>
      </div>
      {!folded && facts && (
        <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-meta text-muted" data-testid="page-facts">
          {facts}
        </div>
      )}
    </header>
  );
}

export function PageShell({
  kind,
  tabs,
  header,
  render,
  footer,
  folded = false,
  focusTabsRequest = 0,
  testId = "page",
}: {
  kind: PageKind;
  tabs: PageTab[];
  header: ReactNode;
  /** A tab's body; `visible` while it's the shown one (previews pause). */
  render: (tab: PageTabId, visible: boolean) => ReactNode;
  /** Under each tab: when it was updated and where its numbers come from. */
  footer?: (tab: PageTab) => ReactNode;
  folded?: boolean;
  /** Bumped when focus should move to the page's selected tab. */
  focusTabsRequest?: number;
  testId?: string;
}) {
  const [picked, setPicked] = useState<PageTabId>(() => rememberedTab(kind, tabs));
  // A tab that went away (nothing left to show) falls back to the body.
  const tab = tabs.some((entry) => entry.id === picked) ? picked : tabs[0]!.id;
  const tabsRef = useRef<HTMLDivElement | null>(null);

  const choose = (next: PageTabId) => {
    setPicked(next);
    rememberTab(kind, next);
  };
  // AJ-E1103: a tab the user picked (not one the app moved to).
  const pick = (next: PageTabId) => {
    statsEvent("ui.inspector_tab", { selection: kind, tab: next });
    choose(next);
  };

  // ⌘⇧H: hand over lives on the body.
  useEffect(() => {
    const toBody = () => setPicked(tabs[0]!.id);
    window.addEventListener("aij:focus-handover", toBody);
    return () => window.removeEventListener("aij:focus-handover", toBody);
  });

  // ⌥⌘1…n picks a tab; focus follows.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!matches("inspector-tab", event)) return;
      const index = Number(event.code.startsWith("Digit") ? event.code.slice(5) : event.key);
      const next = tabs[index - 1];
      if (!next) return;
      event.preventDefault();
      statsEvent("ui.shortcut_used", { shortcut: "inspector_tab" });
      pick(next.id);
      tabsRef.current?.querySelectorAll<HTMLButtonElement>("[role='tab']")[index - 1]?.focus();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // A focus request goes to the selected tab, or to the page's name on a
  // page that is its body alone, once it's there to take it.
  const pendingFocus = useRef(0);
  const rootRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (focusTabsRequest > 0) pendingFocus.current = focusTabsRequest;
  }, [focusTabsRequest]);
  useEffect(() => {
    if (pendingFocus.current === 0) return;
    const target =
      tabsRef.current?.querySelector<HTMLElement>("[role='tab'][aria-selected='true']") ??
      rootRef.current?.querySelector<HTMLElement>("[data-testid='page-name']");
    if (target) {
      target.focus();
      pendingFocus.current = 0;
    }
  });

  const strip = tabs.length > 1;
  return (
    // WCAG 1.4.10: when the window is too short for the header and a
    // usable body (200 % zoom, a short window), the page scrolls as a
    // whole instead of squeezing its body to nothing.
    // The shared page frame (0.1.5): header, tabs and body in one centred
    // 1440 px column, so a wide window never strands the header.
    <div ref={rootRef} className="mx-auto flex h-full min-h-0 w-full max-w-content flex-col gap-3 overflow-y-auto" data-testid={testId} data-page-kind={kind} data-folded={folded ? "true" : undefined}>
      {header}
      {strip && (
        <div ref={tabsRef} role="tablist" hidden={folded} aria-label={`${PAGE_LABELS[kind]} tabs`} className="flex flex-wrap gap-1 border-b border-hairline pb-2">
          {tabs.map((entry, index) => (
            <button
              key={entry.id}
              type="button"
              role="tab"
              id={`page-tab-${entry.id}`}
              data-testid={`page-tab-${entry.id}`}
              aria-selected={tab === entry.id}
              aria-controls={`page-panel-${entry.id}`}
              tabIndex={tab === entry.id ? 0 : -1}
              onClick={() => pick(entry.id)}
              onKeyDown={(event) => {
                if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
                event.preventDefault();
                const next = (index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
                choose(tabs[next]!.id);
                tabsRef.current?.querySelectorAll<HTMLButtonElement>("[role='tab']")[next]?.focus();
              }}
              className={
                "rounded-lg px-2.5 py-1 text-meta font-medium outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (tab === entry.id ? "bg-accent-soft text-accent-hi" : "text-muted hover:bg-surface hover:text-ink")
              }
            >
              {entry.label}
            </button>
          ))}
        </div>
      )}
      <div className="flex min-h-64 flex-1 flex-col">
        {tabs.map((entry) => (
          <section
            key={entry.id}
            role={strip ? "tabpanel" : undefined}
            id={`page-panel-${entry.id}`}
            aria-labelledby={strip ? `page-tab-${entry.id}` : undefined}
            data-testid={`page-panel-${entry.id}`}
            hidden={tab !== entry.id}
            // A panel scrolls: reachable by keyboard (APG tabs; WCAG 2.1.1).
            tabIndex={0}
            className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto rounded-lg outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            {render(entry.id, tab === entry.id)}
            {footer?.(entry)}
          </section>
        ))}
      </div>
    </div>
  );
}
