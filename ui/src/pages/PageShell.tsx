/**
 * AJ-E1533 — the one page every item opens in (pages/registry.ts): an
 * identity header (PageHeader), then the page's tabs, the first its body. Panels
 * toggle `hidden` (a terminal never remounts on a tab switch); the tab
 * a type of page was left on is remembered for that type only.
 */
import { useCallback, useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { useScrollMemory } from "../lib/scroll-memory";
import { useHandoverPage } from "../lib/handover-focus";
import { matches } from "../lib/shortcuts";
import { statsEvent } from "../lib/ipc/usage-stats";
import { PAGE_LABELS, PAGE_TAB_OF, rememberTab, rememberedTab, rememberedTabId, type PageKind, type PageTab, type PageTabId } from "./registry";

const SHOW_TAB = "aij:page-tab";

/** Shows another tab of the open page, when it has it. */
export function showTab(tab: PageTabId): void {
  window.dispatchEvent(new CustomEvent<PageTabId>(SHOW_TAB, { detail: tab }));
}

/** The last focus request a page took (each is taken once, by the page
 * shown when it's made, never by the pages opened after it). */
let focusHonoured = 0;

export function PageShell({
  kind,
  tabs,
  header,
  render,
  footer,
  folded = false,
  focusTabsRequest = 0,
  testId = "page",
  subject,
  tabsKnown = true,
  emptyTabs,
  switchKey,
  notice,
}: {
  /** Tabs this item has nothing in (a session in no pool: Pools). One
   * remembered from another item isn't where this one opens (the 0.4.1
   * persona walks: sessions opened on "In no pool now"); a tab the user
   * picks here stays. */
  emptyTabs?: readonly PageTabId[];
  /** D-87: what is about this item alone (a session's own account
   * running out), inline under its header; never another item's. */
  notice?: ReactNode;
  /** A session's page: the key its switch list answers to
   * (`switchSourceOf`, or AI Juggler's session id). */
  switchKey?: string;
  kind: PageKind;
  /** 0.3.4: the item shown (its id): the page and each of its tabs open
   * where they were left for it; another item starts at the top. */
  subject?: string;
  /** False while tabs that show only when the item has some (a session's
   * Messages, Tasks) may still come: the tab this type of page was left
   * on is kept for them until then, the body shown meanwhile. */
  tabsKnown?: boolean;
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
  // The remembered tab even when it isn't here yet: one that shows only
  // when the item has some comes once that's known.
  const [picked, setPicked] = useState<PageTabId>(() => rememberedTabId(kind) ?? rememberedTab(kind, tabs));
  // A tab that went away (nothing left to show) falls back to the body.
  const tab = tabs.some((entry) => entry.id === picked) ? picked : tabs[0]!.id;
  // Once every tab is known, a remembered one this item hasn't is let go
  // (a message arriving later doesn't pull the page off its body).
  const settled = useRef(false);
  useEffect(() => {
    if (!tabsKnown || settled.current) return;
    settled.current = true;
    if (picked !== tab) setPicked(tab);
  }, [tabsKnown, picked, tab]);
  const tabsRef = useRef<HTMLDivElement | null>(null);

  const choose = (next: PageTabId) => {
    setPicked(next);
    rememberTab(kind, next);
  };
  // A remembered tab this item has nothing in lets go to the body, until
  // the user picks one for this item.
  const pickedFor = useRef<string | undefined | null>(null);
  const emptyKey = emptyTabs?.join(" ") ?? "";
  useEffect(() => {
    if (pickedFor.current !== subject && emptyKey.split(" ").includes(picked)) setPicked(tabs[0]!.id);
  }, [emptyKey, picked, tabs, subject]);
  // AJ-E1103: a tab the user picked (not one the app moved to).
  const pick = (next: PageTabId) => {
    statsEvent("ui.inspector_tab", { selection: kind, tab: next });
    pickedFor.current = subject;
    choose(next);
  };

  // ⌘⇧H: a session's switch list lives on its body; from another tab
  // the body opens and the list takes the keyboard as it mounts. Only
  // for this page's own session (`switchKey`).
  useHandoverPage(() => setPicked(tabs[0]!.id), kind === "session" || kind === "conversation" ? switchKey : undefined);

  // E19: a tab's link asks for another of this page's tabs (a task's
  // thread opens in Messages).
  useEffect(() => {
    const to = (event: Event) => {
      const next = (event as CustomEvent<PageTabId>).detail;
      if (tabs.some((entry) => entry.id === next)) choose(next);
    };
    window.addEventListener(SHOW_TAB, to);
    // AJ-E3104: a notice about a page of this kind, open now.
    const of = (event: Event) => {
      const { kind: asked, tab: next } = (event as CustomEvent<{ kind: PageKind; tab: PageTabId }>).detail;
      if (asked === kind && tabs.some((entry) => entry.id === next)) choose(next);
    };
    window.addEventListener(PAGE_TAB_OF, of);
    return () => {
      window.removeEventListener(SHOW_TAB, to);
      window.removeEventListener(PAGE_TAB_OF, of);
    };
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
  const keep = useScrollMemory(subject === undefined ? null : `page:${kind}:${subject}`);
  const root = useCallback(
    (element: HTMLDivElement | null) => {
      rootRef.current = element;
      return keep(element);
    },
    [keep],
  );
  useEffect(() => {
    // Each request once, though the page it opens is a new shell.
    if (focusTabsRequest > focusHonoured) pendingFocus.current = focusHonoured = focusTabsRequest;
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
    <div ref={root} className="mx-auto flex h-full min-h-0 w-full max-w-content flex-col gap-3 overflow-y-auto" data-testid={testId} data-page-kind={kind} data-folded={folded ? "true" : undefined}>
      {header}
      {notice}
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
                // The user's pick, as a click is: on a session in no pool,
                // Pools reached by ← → stays (CI #218's tab walk).
                pick(tabs[next]!.id);
                tabsRef.current?.querySelectorAll<HTMLButtonElement>("[role='tab']")[next]?.focus();
              }}
              className={
                "rounded-lg px-2.5 py-1 text-meta font-medium outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (tab === entry.id ? "bg-accent-soft text-accent-hi" : "text-muted hover:bg-surface hover:text-ink")
              }
            >
              {entry.label}
              {/* The chosen tab's count at full strength (axe: color-contrast
                  on its tinted surface); the others' dimmer than their name. */}
              {entry.count !== undefined && entry.count > 0 && <span className={"ml-1 tabular-nums" + (tab === entry.id ? "" : " opacity-70")}> {entry.count}</span>}
            </button>
          ))}
        </div>
      )}
      <div className="flex min-h-64 flex-1 flex-col">
        {tabs.map((entry) => (
          <TabPanel key={entry.id} id={entry.id} strip={strip} shown={tab === entry.id} scrollKey={subject === undefined ? null : `page:${kind}:${subject}:${entry.id}`}>
            {render(entry.id, tab === entry.id)}
            {footer?.(entry)}
          </TabPanel>
        ))}
      </div>
    </div>
  );
}

/** One tab's body: it scrolls, and opens where it was left (0.3.4). */
function TabPanel({ id, strip, shown, scrollKey, children }: { id: PageTabId; strip: boolean; shown: boolean; scrollKey: string | null; children: ReactNode }) {
  const scroll = useScrollMemory<HTMLElement>(scrollKey, { active: shown });
  return (
    <section
      ref={scroll}
      role={strip ? "tabpanel" : undefined}
      id={`page-panel-${id}`}
      aria-labelledby={strip ? `page-tab-${id}` : undefined}
      data-testid={`page-panel-${id}`}
      hidden={!shown}
      // A panel scrolls: reachable by keyboard (APG tabs; WCAG 2.1.1).
      tabIndex={0}
      className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto rounded-lg outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      {children}
    </section>
  );
}

/** Its children once the tab has been shown, kept after (a hidden tab
 * reads nothing until it's opened). */
export function WhenShown({ visible, children }: { visible: boolean; children: ReactNode }): JSX.Element | null {
  const [shown, setShown] = useState(visible);
  useEffect(() => {
    if (visible) setShown(true);
  }, [visible]);
  return shown || visible ? <>{children}</> : null;
}
