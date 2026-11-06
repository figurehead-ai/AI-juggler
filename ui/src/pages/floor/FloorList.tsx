/**
 * AJ-E3512 (D-78): the Floor list — the same facts as the Floor, for a
 * screen reader or the keyboard (accessibility isn't a mode; it's the same
 * information). Grouped by floor, then App, in the Floor's order. Each
 * session's state is a glyph and a word; Show on the Floor opens the
 * Floor on that App's floor. New "waiting on you" sessions are announced
 * politely, once each. Reached from the Floor button's menu and ⌘K.
 * The 0.4 review: a search, a "Waiting on you" chip, and each floor in
 * the shared paged list under its one header (floor-list-model.ts,
 * FloorRows.tsx).
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { floorOpen, floorSnapshot, onFloorChanged, type FloorSession, type FloorSnapshot } from "../../lib/ipc/floor";
import { floorPlaces, floorSaid, floorShowApp, type FloorPlaceInfo } from "../../lib/ipc/floor-controls";
import { useKeys } from "../../lib/use-keys";
import { floorRows, groups, summary, waitingCount } from "./floor-list-model";
import { FloorSection } from "./FloorRows";

export { GLYPH, groups, summary, waitingFirst, floorColumns, floorRows, type FloorGroup } from "./floor-list-model";

/** As the Floor's card: read after resting this long on a row… */
export const SAID_AFTER = 250;
/** …and kept this long after leaving it. */
export const SAID_STAYS = 300;

/** AJ-E3517's gate (the list carries the card's words): the session a
 * row rests on (focus or pointer), and what it asks or did last, read for
 * that one row only after a rest; gone a moment after leaving. */
function useRowSaid(): { said: { title: string; words: string } | null; look: (session: FloorSession | null) => void } {
  const [said, setSaid] = useState<{ title: string; words: string } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const asked = useRef(0);
  useEffect(() => () => void (timer.current && clearTimeout(timer.current)), []);
  const look = (session: FloorSession | null) => {
    if (timer.current) clearTimeout(timer.current);
    const turn = ++asked.current;
    if (!session) {
      timer.current = setTimeout(() => turn === asked.current && setSaid(null), SAID_STAYS);
      return;
    }
    timer.current = setTimeout(() => {
      void floorSaid(session.key).then(
        (got) => {
          if (turn !== asked.current) return;
          const words = got.asks ? `Asks: ${got.asks}` : got.step ? `Last: ${got.step}` : null;
          setSaid(words ? { title: session.title, words } : null);
        },
        () => turn === asked.current && setSaid(null),
      );
    }, SAID_AFTER);
  };
  return { said, look };
}

export function FloorList(): JSX.Element {
  const [snapshot, setSnapshot] = useState<FloorSnapshot | null>(null);
  const [places, setPlaces] = useState<FloorPlaceInfo[]>([]);
  const [said, setSaid] = useState("");
  const known = useRef<Set<string> | null>(null);
  const keys = useKeys("floor");
  const rowSaid = useRowSaid();
  useEffect(() => {
    void floorSnapshot().then(setSnapshot, () => setSnapshot(null));
    void floorPlaces().then(setPlaces, () => setPlaces([]));
    return onFloorChanged((next) => {
      setSnapshot(next);
      void floorPlaces().then(setPlaces, () => undefined);
    });
  }, []);
  // New "waiting on you" sessions, said politely, once each. Sessions AI
  // Juggler runs ("s:" keys) are already said by the main window's
  // AttentionAnnouncer; this says the desktop apps' conversations.
  useEffect(() => {
    if (!snapshot) return;
    const waiting = snapshot.apps.flatMap((app) => app.sessions.filter((s) => s.state === "asks" && !s.key.startsWith("s:")).map((s) => ({ key: s.key, words: `${s.title} in ${app.name} asks you.` })));
    if (known.current) {
      const fresh = waiting.filter((entry) => !known.current!.has(entry.key));
      if (fresh.length > 0) setSaid(fresh.map((entry) => entry.words).join(" "));
    }
    known.current = new Set(waiting.map((entry) => entry.key));
  }, [snapshot]);
  const list = useMemo(() => (snapshot ? groups(snapshot, places) : []), [snapshot, places]);
  const [query, setQuery] = useState("");
  const [waitingOnly, setWaitingOnly] = useState(false);
  const waiting = waitingCount(list);
  const now = snapshot?.at_ms ?? Date.now();
  const show = (appId: string) => void floorShowApp(appId).catch((cause: unknown) => showToast({ id: "floor-unavailable", tone: "warning", message: errorText(cause) }));
  const narrowed = query.trim() !== "" || waitingOnly;
  return (
    <section aria-labelledby="floor-list-title" className="mx-auto flex w-full max-w-content flex-col gap-5 pb-8" data-testid="floor-list">
      <header className="flex flex-wrap items-end gap-3">
        <div className="min-w-0 flex-1">
          <h2 id="floor-list-title" className="text-title text-ink">
            The Floor
          </h2>
          <p className="mt-1 text-meta text-ink-2" data-testid="floor-list-summary">
            {snapshot ? summary(list) : "Reading the office…"}
          </p>
        </div>
        <Button size="sm" variant="secondary" aria-keyshortcuts={keys.aria} onClick={() => void floorOpen().catch((cause: unknown) => showToast({ id: "floor-unavailable", tone: "warning", message: errorText(cause) }))}>
          Open the Floor {keys.words}
        </Button>
      </header>
      <p className="sr-only" aria-live="polite" data-testid="floor-list-said">
        {said}
      </p>
      {snapshot && list.length === 0 && <p className="rounded-xl border border-hairline px-4 py-5 text-center text-body text-ink-2">No Apps on the Floor yet. They come as your sessions start.</p>}
      {list.length > 0 && (
        <div className="flex flex-wrap items-center gap-2" data-testid="floor-list-filters">
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search sessions, projects and Apps"
            aria-label="Search the Floor list"
            className="h-9 min-w-[14rem] flex-1 rounded-lg border border-hairline bg-surface px-3 text-body text-ink placeholder:text-muted"
            data-testid="floor-list-search"
          />
          <button
            type="button"
            aria-pressed={waitingOnly}
            onClick={() => setWaitingOnly(!waitingOnly)}
            disabled={waiting === 0 && !waitingOnly}
            className={
              "inline-flex h-8 items-center gap-1.5 rounded-full border px-3 text-meta outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-50 " +
              (waitingOnly ? "border-accent/45 bg-accent-soft text-ink" : "border-hairline bg-surface text-muted hover:text-ink")
            }
            data-testid="floor-list-waiting"
          >
            {/* Its name is its words ("Waiting on you 2"), as seen. */}
            Waiting on you{" "}
            <span className={"font-semibold tabular-nums " + (waiting > 0 ? "text-warning" : "")}>{waiting}</span>
          </button>
        </div>
      )}
      {snapshot && list.map((group) => <FloorSection key={group.floor} group={group} snapshot={snapshot} now={now} query={query} waitingOnly={waitingOnly} onShow={show} onLook={rowSaid.look} />)}
      {narrowed && !list.some((group) => floorRows(group, query, waitingOnly).length > 0) && (
        <p className="rounded-xl border border-hairline px-4 py-5 text-center text-body text-ink-2" data-testid="floor-list-none">
          {query.trim() ? `Nothing on the Floor matches “${query.trim()}”${waitingOnly ? " and waits on you" : ""}.` : "Nothing waits on you."}{" "}
          <button type="button" onClick={() => (setQuery(""), setWaitingOnly(false))} className="rounded text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60">
            Show everything
          </button>
        </p>
      )}
      {/* What the row in focus asks or did last: in view at the list's
          foot, and said politely to a screen reader. */}
      <div className="pointer-events-none sticky bottom-3 z-20 flex justify-center" aria-live="polite" data-testid="floor-list-row-said">
        {rowSaid.said && (
          <p className="max-w-full rounded-xl border border-hairline bg-surface px-4 py-2.5 text-meta text-ink shadow-card">
            <span className="font-semibold">{rowSaid.said.title}</span>
            <span className="text-ink-2"> · </span>
            {rowSaid.said.words}
          </p>
        )}
      </div>
    </section>
  );
}
