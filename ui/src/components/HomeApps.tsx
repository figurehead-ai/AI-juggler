/**
 * D-118 (AJ-E3701 R2) — Home's right column, "Your apps": one compact
 * round-2 row per App (an app in a profile, D-62), not tiles. Each row:
 * the App's own icon (D-38) in its tone (muted at rest, in full colour
 * while it needs you, hovered or focused: D-117), its name (its page),
 * the profile's dot and name, "n open", the needs mark when it has needs
 * (L2/L3 counted, the mark's dot in its level's colour; the loud copy is
 * the pinned need's), and a ghost Open (the app forward) or Start (it
 * starts it). Running first, the most recently used first; then a muted
 * "Not running" group (0.3.4, D-64: every remembered App, so one that
 * quits is one click from starting again). Past 8, "Show all".
 */
import { useState, type JSX } from "react";
import { AppIcon } from "./AppIcon";
import { ProfileBadge } from "./ProfileBadge";
import { HomeSection } from "./HomeSection";
import { showToast } from "./Toast";
import { appTone, brokenOf, brokenParts, counted, highest, itemLevel, type AttentionLevel } from "../lib/attention";
import { showGoTo } from "../lib/go-to";
import { conversationsBringForward } from "../lib/ipc/conversations";
import { placesOpen, type KnownPlaceInfo } from "../lib/ipc/places";
import { placeFor, placeWords, useKnownPlaces } from "../lib/places";
import { groupItems, type RailGroup } from "../lib/rail-places";
import { appFolderOf, knownAppWord } from "../lib/rail-apps";
import { appIdOfRailKey, openApp } from "../lib/apps";
import { appName, desktopProfile } from "../lib/app-name";
import type { RailItem } from "../lib/rail-items";

/** Rows shown before "Show all". */
export const ROWS_SHOWN = 8;

/** The places with something open, AI Juggler's own left out (it's here),
 * the most recently used first. */
export function appPlaces(items: RailItem[]): RailGroup[] {
  const groups = groupItems(
    items.filter((item) => item.live && item.place.kind !== "own"),
    "place",
  );
  const last = (group: RailGroup) => Math.max(...group.items.map((item) => item.lastMs));
  return groups.sort((a, b) => last(b) - last(a));
}

function bringForward(pid: number | null | undefined, name: string): void {
  if (pid === null || pid === undefined) return;
  showToast({ id: "go-to", message: `Bringing ${name} forward…` });
  conversationsBringForward(pid).then(showGoTo, () => showToast({ id: "go-to", message: `Couldn't bring ${name} forward` }));
}

function start(place: KnownPlaceInfo, name: string): void {
  const running = place.app_pid !== null;
  showToast({ id: "go-to", message: running ? `Bringing ${name} forward…` : `Starting ${name}…` });
  placesOpen(place.id).then(showGoTo, () => showToast({ id: "go-to", message: running ? `Couldn't bring ${name} forward` : `Couldn't start ${name}` }));
}

const GHOST = "btn-ghost shrink-0 rounded-md px-2.5 py-1 text-meta font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-not-allowed disabled:opacity-50";

/** One row: icon, name and profile line, the needs mark, the action. */
function Row({ id, icon, app, profile, slot, line, needs, level, broken, action, testId, place, lit }: {
  id: string | null;
  icon: JSX.Element;
  app: string;
  profile: string | null;
  slot: string | number;
  line: string;
  needs: number;
  level: AttentionLevel;
  broken: readonly string[];
  action: JSX.Element | null;
  testId: string;
  place: string;
  lit: boolean;
}): JSX.Element {
  const name = appName(app, profile);
  return (
    <li className="tone-host flex min-w-0 items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-surface-2" data-testid={testId} data-place={place} data-level={level} data-lit={lit || undefined}>
      {icon}
      <span className="flex min-w-0 flex-1 flex-col">
        {id === null ? (
          <span className="truncate text-body font-semibold text-ink">{app}</span>
        ) : (
          <button type="button" onClick={() => openApp(id)} title={`Open ${name}'s page`} className="truncate rounded text-left text-body font-semibold text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="home-app-page">
            {app}
            <span className="sr-only">, {name}'s page</span>
          </button>
        )}
        <span className="flex min-w-0 items-center gap-1 text-label text-muted">
          {profile && (
            <>
              <ProfileBadge app={slot} name={profile} />
              <span className="truncate font-medium text-ink-2" title={profile}>{profile}</span>
              <span aria-hidden="true">·</span>
            </>
          )}
          <span className="truncate">{line}</span>
        </span>
      </span>
      {needs > 0 && (
        // D-117: a mark, not a pill: the words in ink-2 by the level's dot.
        <span className="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap text-label font-medium text-ink-2" data-testid="home-app-needs" data-level={level} title="Pinned under Needs you now">
          <span aria-hidden="true" className="level-mark" data-level={level === 3 ? 3 : 2} />
          {needs} {needs === 1 ? "needs" : "need"} you
          {broken.length > 0 && <span className="sr-only">, {brokenParts(broken)}</span>}
        </span>
      )}
      {action}
    </li>
  );
}

function OpenRow({ group, appId }: { group: RailGroup; appId: string | null }): JSX.Element {
  const place = group.place!;
  // D-117: L2 and L3 counted; its mark takes the highest.
  const loud = group.items.filter((item) => counted(itemLevel(item)));
  const level = highest(loud.map(itemLevel));
  const name = appName(place.app, place.profile);
  const count = group.items.length;
  return (
    <Row
      id={appId}
      icon={<AppIcon appId={place.appId} appPid={place.appPid} name={place.app} size={20} className={appTone(level >= 2)} />}
      app={place.app}
      profile={place.profile ?? null}
      slot={place.key}
      line={`${count} open`}
      needs={loud.length}
      level={level}
      broken={brokenOf(loud, itemLevel, (item) => item.need)}
      testId="home-app-tile"
      place={place.key}
      lit={level >= 2}
      action={place.appPid !== null ? <button type="button" className={GHOST} aria-label={`Open ${name}`} onClick={() => bringForward(place.appPid, name)}>Open</button> : null}
    />
  );
}

function KeptRow({ place }: { place: KnownPlaceInfo }): JSX.Element {
  const running = place.app_pid !== null;
  const app = knownAppWord(place);
  const profile = desktopProfile(place.instance, place.profile);
  const name = appName(app, profile);
  const verb = running ? "Open" : "Start";
  // Its state in the words every place line uses (lib/places).
  const line = placeWords(place);
  return (
    <Row
      id={place.id}
      icon={<AppIcon appId={place.app_id} appPid={null} name={app} size={20} className={appTone(false)} />}
      app={app}
      profile={profile}
      slot={place.colour_slot ?? place.id}
      line={line}
      needs={0}
      level={0}
      broken={[]}
      testId={running ? "home-app-kept" : "home-app-closed"}
      place={place.id}
      lit={false}
      action={
        <button type="button" className={GHOST} onClick={() => start(place, name)} disabled={!running && place.cannot_start !== null} title={running ? undefined : (place.cannot_start ?? undefined)} aria-label={`${verb} ${name}`}>
          {verb}
        </button>
      }
    />
  );
}

/** Under 640 px (AJ-E3701 R4): a strip of App icons, each its App forward
 * (or started), its needs a dot on its corner, its name in its own. */
function Strip({ places, kept }: { places: RailGroup[]; kept: KnownPlaceInfo[] }): JSX.Element {
  const cell = "tone-host relative flex size-9 shrink-0 items-center justify-center rounded-lg bg-surface outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60";
  return (
    <ul className="flex gap-2 overflow-x-auto pb-1" data-testid="home-apps-strip">
      {places.map((group) => {
        const place = group.place!;
        const name = appName(place.app, place.profile);
        const level = highest(group.items.filter((item) => counted(itemLevel(item))).map(itemLevel));
        return (
          <li key={group.key}>
            <button type="button" className={cell} aria-label={`Open ${name}${level >= 2 ? ", needs you" : ""}`} title={name} onClick={() => bringForward(place.appPid, name)} data-testid="home-app-tile" data-place={place.key} data-level={level}>
              <AppIcon appId={place.appId} appPid={place.appPid} name={place.app} size={24} className={appTone(level >= 2)} />
              {level >= 2 && <span aria-hidden="true" className="level-mark absolute right-0.5 top-0.5" data-level={level} />}
            </button>
          </li>
        );
      })}
      {kept.map((place) => {
        const name = appName(knownAppWord(place), desktopProfile(place.instance, place.profile));
        return (
          <li key={place.id}>
            <button type="button" className={cell + " opacity-70"} aria-label={`${place.app_pid !== null ? "Open" : "Start"} ${name}`} title={name} onClick={() => start(place, name)} disabled={place.app_pid === null && place.cannot_start !== null} data-testid={place.app_pid !== null ? "home-app-kept" : "home-app-closed"}>
              <AppIcon appId={place.app_id} appPid={null} name={knownAppWord(place)} size={24} className={appTone(false)} />
            </button>
          </li>
        );
      })}
    </ul>
  );
}

export function HomeApps({ items, strip = false }: { items: RailItem[]; strip?: boolean }): JSX.Element | null {
  const [all, setAll] = useState(false);
  const known = useKnownPlaces();
  const places = appPlaces(items);
  const shownPids = new Set(places.map((group) => group.place?.appPid ?? null));
  // Remembered with no open row: running ones (nothing open yet) with the
  // running; down ones under "Not running", kept alive first, then the
  // most recently seen.
  const kept = (known ?? [])
    .filter((place) => place.app_pid === null || !shownPids.has(place.app_pid))
    .sort((a, b) => Number(b.keep_alive) - Number(a.keep_alive) || Number(a.found) - Number(b.found) || b.seen_ms - a.seen_ms);
  const runningKept = kept.filter((place) => place.app_pid !== null);
  const down = kept.filter((place) => place.app_pid === null);
  const total = places.length + kept.length;
  if (total === 0) return null;
  if (strip) {
    return (
      <HomeSection id="home-apps" label="Your apps" count={total} className="min-w-0">
        <Strip places={places} kept={kept} />
      </HomeSection>
    );
  }
  const cut = !all && total > ROWS_SHOWN + 1;
  const room = cut ? ROWS_SHOWN : total;
  const folderOf = (group: RailGroup) => appFolderOf(group, placeFor(known, group.place?.appPid ?? null));
  const running = [...places.map((group) => ({ group })), ...runningKept.map((place) => ({ place }))].slice(0, room);
  const closed = down.slice(0, Math.max(0, room - running.length));
  return (
    <HomeSection id="home-apps" label="Your apps" count={total} className="min-w-0">
      <div data-testid="home-apps" className="flex flex-col gap-1 rounded-xl bg-surface p-2">
        <ul className="flex flex-col">
          {running.map((row) =>
            "group" in row ? (
              <OpenRow key={row.group.key} group={row.group} appId={row.group.place ? appIdOfRailKey(row.group.place.key, folderOf(row.group), known) : null} />
            ) : (
              <KeptRow key={row.place.id} place={row.place} />
            ),
          )}
        </ul>
        {closed.length > 0 && (
          <>
            <p className="px-2 pt-1 text-label text-muted" data-testid="home-apps-down">Not running</p>
            <ul className="flex flex-col">
              {closed.map((place) => <KeptRow key={place.id} place={place} />)}
            </ul>
          </>
        )}
        {total > ROWS_SHOWN + 1 && (
          <button type="button" aria-expanded={all} onClick={() => setAll((open) => !open)} data-testid="home-apps-more" className="self-start rounded px-2 py-1 text-label text-ink-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
            {all ? "Show fewer" : `Show all ${total}`}
          </button>
        )}
      </div>
    </HomeSection>
  );
}
