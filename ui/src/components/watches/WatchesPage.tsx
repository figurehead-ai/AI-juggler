/**
 * AJ-E3816 (D-132, D-126, ADR-0036 §2.6) — the Watches page, to the
 * approved mock-up: one place to see and manage every watch, with or
 * without AI. The head (the count of the limit, what a watch is, New
 * watch and Ask Bot), the quiet-hours line when the user has quiet hours,
 * the watching list (each watch's echo, its target, chips, where it
 * stands, Edit, Pause, ⋯), and Ended, folded and paged. Past eight
 * watches the list gets search, filters with counts and sort, and goes
 * compact, ten rows then twenty more. At the limit the amber alert shows
 * only once New watch is pressed. New and Edit open one form in place.
 *
 * Instant and paged: the live watches are one bounded read (at most 50)
 * remembered between visits; Ended is read a page at a time once opened.
 */
import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../primitives";
import { showToast } from "../Toast";
import { usePagedList, refreshPaged } from "../../lib/paged";
import { errorText } from "../../lib/bridge";
import { notificationPrefsGet } from "../../lib/ipc/settings";
import { onWatchesIntent, takeWatchesIntent, type WatchesIntent } from "../../lib/watches-open";
import { watchCreate, watchesPage, watchesSummary, watchPause, watchRemove, watchRestore, watchUpdate, type WatchDraftInfo, type WatchInfo, type WatchSummaryInfo } from "../../lib/ipc/watch-view";
import type { NotificationPrefsInfo } from "../../types/generated/NotificationPrefsInfo";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { WatchForm } from "./WatchForm";
import type { WatchRowActions } from "./WatchRow";
import type { EndedActions } from "./WatchEndedRow";
import { WatchesEmpty, startersFor, type Starter } from "./WatchesEmpty";
import { LimitAlert, QuietLine, WatchesHead, type ToolbarState } from "./WatchesHead";
import { watchesChanged } from "./WatchDoor";
import { EndedSection, WatchingSection } from "./WatchesLists";
import { stateFrom } from "./watch-form-state";
import { draftOf, nextMidnight, pickerLists, quietHours, targetBook, type TargetOption, type TargetSources } from "./watches-view";
import { PAGE, TOAST } from "./watches-words";

type View = { kind: "list" } | { kind: "form"; mode: "new" | "edit"; draft: WatchDraftInfo | null; editing: WatchInfo | null };
type Only = { target: WatchTarget; name: string } | null;

const EMPTY_DRAFT: WatchDraftInfo = { targets: [], matching: "each", conditions: [], lifetime: { kind: "unsaid" }, evenInQuietHours: false };

function useSummary(): [WatchSummaryInfo | null, () => void] {
  const [summary, setSummary] = useState<WatchSummaryInfo | null>(null);
  const read = useCallback(() => {
    void watchesSummary().then(setSummary, () => setSummary(null));
  }, []);
  useEffect(read, [read]);
  return [summary, read];
}

function usePrefs(): NotificationPrefsInfo | null {
  const [prefs, setPrefs] = useState<NotificationPrefsInfo | null>(null);
  useEffect(() => {
    void notificationPrefsGet().then(setPrefs, () => setPrefs(null));
  }, []);
  return prefs;
}

/** The starters, worked out only while there are no watches. */
function useStarters(empty: boolean, book: Map<string, TargetOption>): Starter[] {
  const [starters, setStarters] = useState<Starter[]>([]);
  const lists = useMemo(() => pickerLists(book, ""), [book]);
  useEffect(() => {
    let on = true;
    if (empty) void startersFor(lists.openNow, lists.recent.find((o) => o.kind === "account") ?? null).then((got) => on && setStarters(got));
    return () => {
      on = false;
    };
  }, [empty, lists]);
  return starters;
}

/** "all of “api” and “web” finish": the echo's first sentence, as a toast names it. */
function shortWords(info: WatchInfo): string {
  return info.echo.head
    .map((s) => s.text)
    .join("")
    .replace(/^Tell you when /, "")
    .replace(/\.$/, "");
}

const toDay = (draft: WatchDraftInfo): WatchDraftInfo["lifetime"] => (draft.lifetime.kind === "until" ? { kind: "today" } : draft.lifetime);

const editOf = (info: WatchInfo): View => ({ kind: "form", mode: "edit", draft: draftOf(info.watch, nextMidnight(Date.now())), editing: info });

/** What a door asks for, as the page's view (a watch to edit is read first). */
function viewOfIntent(intent: WatchesIntent | null): View | null {
  if (intent?.kind === "new") return { kind: "form", mode: "new", draft: intent.draft ?? (intent.target ? { ...EMPTY_DRAFT, targets: [intent.target] } : null), editing: null };
  if (intent?.kind === "list" || intent?.kind === "filter") return { kind: "list" };
  return null;
}

/** The doors: the intent the page opened on, and any while it is open. */
function useIntents(first: WatchesIntent | null, setView: (view: View) => void, setOnly: (only: Only) => void): void {
  const apply = useRef((intent: WatchesIntent | null) => {
    const view = viewOfIntent(intent);
    if (view) setView(view);
    if (intent?.kind === "filter") setOnly({ target: intent.target, name: intent.name });
    if (intent?.kind === "edit")
      void watchesPage("all", { cursor: null, limit: 200 }).then((page) => {
        const hit = page.items.find((w) => w.watch.id === intent.id);
        if (hit) setView(editOf(hit));
      });
  });
  useEffect(() => {
    if (first?.kind === "edit") apply.current(first);
    return onWatchesIntent(() => apply.current(takeWatchesIntent()));
  }, [first]);
}

/** The page's writes, each by the watch's id, then the lists read again. */
function useWatchActions(input: { refresh: () => void; book: Map<string, TargetOption>; onGoTo: (target: WatchTarget) => void; openNew: (draft: WatchDraftInfo | null) => void; setView: (view: View) => void }): { row: WatchRowActions; ended: EndedActions; remove: (info: WatchInfo) => void } {
  const { refresh, book } = input;
  const failed = (error: unknown) => showToast({ message: TOAST.failed(errorText(error)), tone: "error" });
  const remove = (info: WatchInfo) =>
    void watchRemove(info.watch.id).then(() => {
      refresh();
      showToast({
        id: `watch-removed-${info.watch.id}`,
        message: TOAST.removed(shortWords(info)),
        tone: "success",
        undo: { onUndo: () => watchRestore(info.watch.id).then(refresh), undoing: TOAST.undoing, done: TOAST.undone, failed: TOAST.undoFailed },
        testId: "watch-undo",
      });
    }, failed);
  const update = (info: WatchInfo, draft: WatchDraftInfo, words: string) =>
    void watchUpdate(info.watch.id, draft).then(() => {
      refresh();
      showToast({ message: words, tone: "success" });
    }, failed);
  const draftNow = (info: WatchInfo) => draftOf(info.watch, nextMidnight(Date.now()));
  const row: WatchRowActions = {
    edit: (info) => input.setView(editOf(info)),
    pause: (info, paused) => void watchPause(info.watch.id, paused).then(refresh, failed),
    quiet: (info, on) => update(info, { ...draftNow(info), evenInQuietHours: on }, on ? TOAST.quietOn : TOAST.quietOff),
    goTo: (info) => input.onGoTo(info.watch.targets[0]!),
    similar: (info) => input.openNew(draftNow(info)),
    remove,
  };
  const ended: EndedActions = {
    forward: (info, way) => {
      const draft = draftNow(info);
      if (way.kind === "app") input.openNew({ ...draft, targets: [{ kind: "app", id: way.appId }], lifetime: { kind: "unsaid" }, matching: "each" });
      else if (way.kind === "retarget") input.setView({ kind: "form", mode: "edit", draft: { ...draft, targets: draft.targets.filter((t) => book.has(t.kind === "any" ? "any" : `${t.kind}:${t.id}`)) }, editing: info });
      else update(info, { ...draft, lifetime: way.kind === "today" ? { kind: "today" } : toDay(draft) }, TOAST.made);
    },
    remove,
  };
  return { row, ended, remove };
}

/** New or Edit, in place of the list. */
function FormView({ view, book, quiet, count, limit, now, onClose, onSaved, remove }: { view: Extract<View, { kind: "form" }>; book: Map<string, TargetOption>; quiet: { from: string; to: string } | null; count: number; limit: number; now: number; onClose: () => void; onSaved: () => void; remove: (info: WatchInfo) => void }): JSX.Element {
  const initial = stateFrom(view.draft ?? EMPTY_DRAFT, now);
  // Finishes, the common choice, is picked already (the mock's).
  if (view.mode === "new" && initial.codes.length === 0) initial.codes = ["finishes"];
  const editing = view.editing;
  return (
    <WatchForm
      key={editing?.watch.id ?? JSON.stringify(view.draft)}
      mode={view.mode}
      initial={initial}
      editing={editing}
      book={book}
      quiet={quiet}
      count={count}
      limit={limit}
      now={now}
      onCancel={onClose}
      onRemove={
        editing
          ? () => {
              remove(editing);
              onClose();
            }
          : undefined
      }
      onSave={async (draft) => {
        await (editing ? watchUpdate(editing.watch.id, draft) : watchCreate(draft));
        onSaved();
        onClose();
        showToast({ message: editing ? TOAST.saved : TOAST.made, tone: "success" });
      }}
    />
  );
}

function LoadState({ live }: { live: { error: unknown; loading: boolean; refresh: () => void } }): JSX.Element | null {
  if (live.error !== null)
    return (
      <div className="flex items-center gap-3 rounded-xl bg-surface-2 px-3.5 py-3 text-meta text-ink-2" role="alert">
        <span className="flex-1">{errorText(live.error).includes("unavailable") ? PAGE.unavailable : PAGE.failed}</span>
        <Button size="sm" onClick={live.refresh}>
          {PAGE.retry}
        </Button>
      </div>
    );
  return (
    <output aria-label={PAGE.loading} className="flex flex-col gap-2" data-testid="watches-skeleton">
      {[0, 1, 2, 3].map((i) => (
        <span key={i} className="block h-[66px] animate-pulse rounded-xl bg-skeleton/60" />
      ))}
    </output>
  );
}

/** The counts the page shows: the backend's, else what the list holds. */
function countsOf(summary: WatchSummaryInfo | null, live: readonly WatchInfo[]) {
  const paused = summary ? summary.paused : live.filter((w) => w.state === "paused").length;
  const watching = summary ? summary.watching : live.length - paused;
  return {
    watching,
    paused,
    count: watching + paused,
    limit: summary?.limit ?? 50,
    ended: summary?.ended ?? 0,
    toldToday: summary?.toldToday ?? 0,
    never: summary?.neverTold ?? 0,
    breakThrough: summary?.evenInQuietHours ?? 0,
  };
}

export function WatchesPage({ sources, onGoTo, onAskBot, onQuietHours }: { sources: TargetSources; onGoTo: (target: WatchTarget) => void; onAskBot: (() => void) | null; onQuietHours: () => void }): JSX.Element {
  const now = sources.now;
  const book = useMemo(() => targetBook(sources), [sources]);
  const live = usePagedList<WatchInfo>("watches:live", (request) => watchesPage("live", request), { pageSize: 50, keyOf: (row) => row.watch.id });
  const [endedOpen, setEndedOpen] = useState(false);
  const [summary, readSummary] = useSummary();
  const quiet = quietHours(usePrefs());
  const [first] = useState(takeWatchesIntent);
  const [view, setView] = useState<View>(() => viewOfIntent(first) ?? { kind: "list" });
  const [only, setOnly] = useState<Only>(() => (first?.kind === "filter" ? { target: first.target, name: first.name } : null));
  useIntents(first, setView, setOnly);
  const [limitShown, setLimitShown] = useState(false);
  const [toolbar, setToolbar] = useState<ToolbarState>({ query: "", filter: "all", sort: "last" });
  const empty = !live.loading && live.items.length === 0 && countsOf(summary, []).ended === 0;
  const starters = useStarters(empty, book);
  const counts = countsOf(summary, live.items);
  const { count, limit } = counts;
  const refresh = () => {
    refreshPaged("watches:");
    live.refresh();
    readSummary();
    watchesChanged();
  };
  const go = setView;
  const openNew = (draft: WatchDraftInfo | null = null) => (count >= limit ? setLimitShown(true) : go({ kind: "form", mode: "new", draft, editing: null }));
  const acts = useWatchActions({ refresh, book, onGoTo, openNew, setView: go });
  if (view.kind === "form") return <FormView view={view} book={book} quiet={quiet} count={count} limit={limit} now={now} onClose={() => go({ kind: "list" })} onSaved={refresh} remove={acts.remove} />;
  const appOf = (sessionId: string) => {
    const session = sources.sessions.find((s) => s.id === sessionId);
    const app = session ? sources.apps.find((a) => a.id === session.launch_recipe_id) : undefined;
    return app ? { id: app.id, name: app.name } : null;
  };
  return (
    <div className="h-full overflow-y-auto" data-testid="watches-page">
      <div className="mx-auto flex w-full max-w-content flex-col gap-[18px] px-3 pb-10 pt-2">
        <WatchesHead count={count} limit={limit} empty={empty} onNew={() => openNew()} onAskBot={onAskBot} />
        {limitShown && count >= limit && (
          <LimitAlert
            limit={limit}
            never={counts.never}
            onShow={() => {
              setToolbar((was) => ({ ...was, filter: "never" }));
              setLimitShown(false);
            }}
          />
        )}
        {(live.error !== null && live.items.length === 0) || live.loading ? (
          <LoadState live={live} />
        ) : empty ? (
          <WatchesEmpty starters={starters} onNew={() => openNew()} onAskBot={onAskBot} onStarter={(draft) => openNew(draft)} />
        ) : (
          <>
            {quiet && <QuietLine from={quiet.from} to={quiet.to} breakThrough={counts.breakThrough} onOpen={onQuietHours} />}
            <WatchingSection
              live={live.items}
              only={only}
              onClearOnly={() => setOnly(null)}
              toolbar={toolbar}
              onToolbar={(next) => setToolbar((was) => ({ ...was, ...next }))}
              counts={counts}
              book={book}
              now={now}
              actions={acts.row}
              quietTo={quiet?.to ?? null}
            />
            {counts.ended > 0 && <EndedSection count={counts.ended} open={endedOpen} onToggle={() => setEndedOpen((open) => !open)} book={book} now={now} appOf={appOf} actions={acts.ended} />}
          </>
        )}
      </div>
    </div>
  );
}
