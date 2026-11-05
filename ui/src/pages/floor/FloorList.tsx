/**
 * AJ-E3512 (D-78): the Floor list — the same facts as the Floor, for a
 * screen reader or the keyboard (accessibility isn't a mode; it's the same
 * information). Grouped by floor, then App, in the Floor's order. Each
 * session's state is a glyph and a word; Show on the Floor opens the
 * Floor on that App's floor. New "waiting on you" sessions are announced
 * politely, once each. Reached from the Floor button's menu and ⌘K.
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { floorOpen, floorSnapshot, onFloorChanged, type FloorApp, type FloorSession, type FloorSnapshot, type FloorState } from "../../lib/ipc/floor";
import { floorPlaces, floorShowApp, type FloorPlaceInfo } from "../../lib/ipc/floor-controls";
import { agoWords } from "../../lib/time-words";
import { useKeys } from "../../lib/use-keys";

/** Each state's glyph (decoration: the word beside it says the same). */
export const GLYPH: Record<FloorState, string> = { asks: "!", working: "▸", stuck: "?", limit: "⏸", crashed: "✕", done: "✓", idle: "·", ended: "–" };
const WAITING: readonly FloorState[] = ["asks", "stuck", "limit", "crashed", "done"];

export interface FloorGroup {
  floor: number;
  apps: FloorApp[];
}

/** The apps by floor, in the Floor's order; an App the plan doesn't place
 * (left off the Floor) isn't listed. With no plan yet, one floor. */
export function groups(snapshot: FloorSnapshot, places: FloorPlaceInfo[]): FloorGroup[] {
  const at = new Map(places.map((place) => [place.app_id, place]));
  const placed = places.length > 0 ? snapshot.apps.filter((app) => at.has(app.id)) : [...snapshot.apps];
  placed.sort((a, b) => (at.get(a.id)?.order ?? 0) - (at.get(b.id)?.order ?? 0));
  const out = new Map<number, FloorApp[]>();
  for (const app of placed) {
    const floor = at.get(app.id)?.floor ?? 0;
    out.set(floor, [...(out.get(floor) ?? []), app]);
  }
  return [...out.entries()].sort((a, b) => a[0] - b[0]).map(([floor, apps]) => ({ floor, apps }));
}

/** "2 floors · 7 Apps · 3 waiting on you". */
export function summary(list: FloorGroup[]): string {
  const apps = list.reduce((n, group) => n + group.apps.length, 0);
  const waiting = list.reduce((n, group) => n + group.apps.reduce((m, app) => m + app.sessions.filter((s) => WAITING.includes(s.state)).length, 0), 0);
  const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  return [plural(list.length, "floor", "floors"), plural(apps, "App", "Apps"), `${waiting} waiting on you`].join(" · ");
}

function accountWords(app: FloorApp, snapshot: FloorSnapshot, now: number): string {
  const account = snapshot.accounts.find((entry) => entry.id === app.account_id);
  const usage = account ? (account.out ? `${account.label} · out of room` : account.left_pct != null ? `${account.label} · ${Math.round(account.left_pct)}% left` : `${account.label} · not measured yet`) : null;
  const hot = app.hot_since_ms != null ? `Busy processor · ${agoWords(app.hot_since_ms, now).replace(" ago", "")}` : null;
  const unused = !app.open && app.sessions.length === 0 && app.last_active_ms != null ? `Unused · ${agoWords(app.last_active_ms, now).replace(" ago", "")}` : null;
  return [hot, unused, usage].filter(Boolean).join(" · ");
}

function SessionRow({ session, onShow }: { session: FloorSession; onShow: () => void }): JSX.Element {
  return (
    <tr className="border-t border-hairline/60" data-testid="floor-list-row" data-state={session.state}>
      <td className="px-3 py-2">
        <span className="flex items-center gap-2">
          <span aria-hidden="true" className={"w-3 text-center font-semibold " + (WAITING.includes(session.state) ? "text-warning" : "text-muted")}>
            {GLYPH[session.state]}
          </span>
          <span className={WAITING.includes(session.state) ? "font-semibold text-ink" : "text-ink"}>{session.word}</span>
        </span>
      </td>
      <th scope="row" className="truncate px-3 py-2 text-left font-normal text-ink" title={session.title}>
        {session.title}
      </th>
      <td className="truncate px-3 py-2 text-ink-2">{session.project ?? "—"}</td>
      <td className="px-3 py-2 text-right tabular-nums text-ink-2">{session.subagents || "—"}</td>
      <td className="px-3 py-2 text-right tabular-nums text-ink-2">{session.tasks || "—"}</td>
      <td className="px-3 py-2 text-right">
        <Button size="sm" variant="ghost" onClick={onShow} aria-label={`Show on the Floor: ${session.title}`}>
          Show on the Floor
        </Button>
      </td>
    </tr>
  );
}

export function FloorList(): JSX.Element {
  const [snapshot, setSnapshot] = useState<FloorSnapshot | null>(null);
  const [places, setPlaces] = useState<FloorPlaceInfo[]>([]);
  const [said, setSaid] = useState("");
  const known = useRef<Set<string> | null>(null);
  const keys = useKeys("floor");
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
  const now = snapshot?.at_ms ?? Date.now();
  const show = (appId: string) => void floorShowApp(appId).catch((cause: unknown) => showToast({ id: "floor-unavailable", tone: "warning", message: errorText(cause) }));
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
      {list.map((group) => (
        <section key={group.floor} aria-labelledby={`floor-${group.floor}`} className="flex flex-col gap-3" data-testid="floor-list-floor">
          <h3 id={`floor-${group.floor}`} className="text-section font-semibold text-ink">
            Floor {group.floor + 1}
          </h3>
          {group.apps.map((app) => (
            <div key={app.id} className="overflow-hidden rounded-xl border border-hairline bg-surface" data-testid="floor-list-app">
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1 bg-surface-2 px-3 py-2">
                <span aria-hidden="true" className="flex size-6 items-center justify-center rounded-md bg-accent-soft text-label font-semibold text-accent-hi">
                  {app.initials}
                </span>
                <h4 className="text-body font-semibold text-ink">{app.name}</h4>
                <span className="text-meta capitalize text-muted">{app.family}</span>
                <span className="ml-auto text-meta text-ink-2">{accountWords(app, snapshot!, now)}</span>
              </div>
              {app.sessions.length === 0 ? (
                <div className="flex items-center gap-3 px-3 py-2 text-meta text-ink-2">
                  <span aria-hidden="true">·</span>
                  <span className="flex-1">
                    No sessions{app.last_active_ms != null ? ` · last active ${agoWords(app.last_active_ms, now)}` : ""}
                  </span>
                  <Button size="sm" variant="ghost" onClick={() => show(app.id)} aria-label={`Show on the Floor: ${app.name}`}>
                    Show on the Floor
                  </Button>
                </div>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full table-fixed text-meta">
                    <caption className="sr-only">{app.name}'s sessions</caption>
                    {/* The same columns in every App's table, so they line up down the page. */}
                    <colgroup>
                      <col className="w-[22%]" />
                      <col />
                      <col className="w-[16%]" />
                      <col className="w-[11%]" />
                      <col className="w-[9%]" />
                      <col className="w-[10.5rem]" />
                    </colgroup>
                    <thead className="text-label text-muted">
                      <tr>
                        <th scope="col" className="px-3 py-1.5 text-left font-semibold">State</th>
                        <th scope="col" className="px-3 py-1.5 text-left font-semibold">Session</th>
                        <th scope="col" className="px-3 py-1.5 text-left font-semibold">Project</th>
                        <th scope="col" className="px-3 py-1.5 text-right font-semibold">Subagents</th>
                        <th scope="col" className="px-3 py-1.5 text-right font-semibold">Tasks</th>
                        <th scope="col" className="px-3 py-1.5"><span className="sr-only">Show</span></th>
                      </tr>
                    </thead>
                    <tbody>
                      {app.sessions.map((session) => (
                        <SessionRow key={session.key} session={session} onShow={() => show(app.id)} />
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          ))}
        </section>
      ))}
    </section>
  );
}
