/**
 * D-64 (0.3.4) — the rail's App grouping is about apps:
 *
 * - an app's sessions that aren't open (idle in its sidebar, or ended)
 *   belong to that app: a CLI session to its tool and folder (its place
 *   says so already), a desktop app's to the app and profile it was last
 *   seen open in (the app's own record, `KnownPlaceInfo.recent`);
 * - the apps with nothing open are listed too, so an app that quits never
 *   vanishes: a desktop app or profile AI Juggler has seen (Start, or
 *   Bring forward while it runs with nothing open), and a CLI app (a tool
 *   on a folder) that has recent sessions of its own (New session there).
 *
 * Pure, so the grouping and order are unit-tested.
 */
import type { KnownPlaceInfo } from "./ipc/places";
import { appName, desktopAppKey, desktopAppWord } from "./app-name";
import { toolSpec, tools } from "./tools";
import { openNow, type RailItem } from "./rail-items";

/** One app with nothing open. */
export interface IdleApp {
  /** The App's key, as the rail's groups key it. */
  key: string;
  kind: "desktop" | "cli";
  /** "Claude app · b-luka", "Claude Code · Work Max". */
  name: string;
  /** Its app ("Claude app", "Claude Code") and profile or folder, as a
   * place's header shows them. */
  app: string;
  profile: string | null;
  /** Its icon's id. */
  appId: string | null;
  /** When it was last active: its newest session, else when it was last
   * seen running (0: never). */
  lastMs: number;
  /** A desktop app running with nothing open (Bring forward). */
  appPid: number | null;
  /** A desktop app's id for Start (`places_open`). */
  placeId: string | null;
  /** Why a desktop app can't be started from here, when it can't. */
  cannotStart: string | null;
  /** A CLI app's folder row (New session there). */
  accountId: string | null;
  /** A CLI app's tool. */
  tool: string | null;
  /** Its sessions that aren't open, newest first. */
  sessions: RailItem[];
}

/** A remembered desktop app's name on screen ("Claude app"), as its open
 * sessions' places say it. */
export function knownAppWord(place: Pick<KnownPlaceInfo, "app" | "app_id">): string {
  return desktopAppWord(place.app, tools().find((tool) => tool.host_app === place.app_id)?.id ?? null);
}

/** A remembered desktop app's App key, as its open sessions are keyed. */
export function knownKey(place: KnownPlaceInfo): string {
  return desktopAppKey(place.app_id, place.app, place.instance);
}

/** Each remembered desktop app's App key, by the sessions last seen open
 * in it (`<tool>/<id>`); the most recent app wins. */
export function appByConversation(known: readonly KnownPlaceInfo[]): Map<string, string> {
  const out = new Map<string, string>();
  const newestFirst = [...known].sort((a, b) => b.seen_ms - a.seen_ms);
  for (const place of newestFirst) {
    for (const key of place.recent) if (!out.has(key)) out.set(key, knownKey(place));
  }
  return out;
}

/** The App an item belongs to: an open one where it's open; one not open
 * where it was last seen open (a desktop app's record), else its place
 * (a CLI session's tool and folder). */
export function appKeyOf(item: RailItem, byConversation: ReadonlyMap<string, string>): string {
  if (item.conversation && !openNow(item)) {
    const desktop = byConversation.get(`${item.conversation.tool}/${item.conversation.id}`);
    if (desktop) return desktop;
  }
  return item.place.key;
}

/** Each App's sessions that aren't open (archived ones left out), newest
 * first. */
export function idleSessionsByApp(items: readonly RailItem[], byConversation: ReadonlyMap<string, string>): Map<string, RailItem[]> {
  const out = new Map<string, RailItem[]>();
  for (const item of items) {
    if (openNow(item) || item.archived || item.need !== null) continue;
    const key = appKeyOf(item, byConversation);
    const list = out.get(key);
    if (list) list.push(item);
    else out.set(key, [item]);
  }
  for (const list of out.values()) list.sort((a, b) => b.lastMs - a.lastMs);
  return out;
}

/** The apps with nothing open, most recently active first: every
 * remembered desktop app, and every CLI app (tool and folder) with recent
 * sessions of its own, whose key no open group has. The rail is for what
 * you use: a folder only a desktop app uses waits for the Apps list. */
export function idleApps({
  openKeys,
  known,
  idle,
}: {
  openKeys: ReadonlySet<string>;
  known: readonly KnownPlaceInfo[];
  idle: ReadonlyMap<string, RailItem[]>;
}): IdleApp[] {
  const out: IdleApp[] = [];
  const desktop = new Set<string>();
  for (const place of known) {
    const key = knownKey(place);
    desktop.add(key);
    if (openKeys.has(key)) continue;
    const sessions = idle.get(key) ?? [];
    const app = knownAppWord(place);
    out.push({
      key,
      kind: "desktop",
      name: appName(app, place.instance),
      app,
      profile: place.instance,
      appId: place.app_id,
      lastMs: Math.max(sessions[0]?.lastMs ?? 0, place.seen_ms),
      appPid: place.app_pid,
      placeId: place.id,
      cannotStart: place.cannot_start,
      accountId: null,
      tool: null,
      sessions,
    });
  }
  for (const [key, sessions] of idle) {
    const first = sessions[0];
    if (!first || openKeys.has(key) || desktop.has(key) || !key.startsWith("cli|")) continue;
    const tool = first.conversation?.tool ?? first.session?.provider ?? null;
    const app = tool ? toolSpec(tool).display_name : first.place.app;
    out.push({
      key,
      kind: "cli",
      name: appName(app, first.place.profile),
      app,
      profile: first.place.profile,
      appId: first.place.appId,
      lastMs: first.lastMs,
      appPid: null,
      placeId: null,
      cannotStart: null,
      accountId: first.accountId,
      tool,
      sessions,
    });
  }
  return out.sort((a, b) => b.lastMs - a.lastMs || a.name.localeCompare(b.name));
}

/** How many inactive sessions the rail shows inline, under an app. */
export const RECENT_INLINE = 3;
/** How many inactive apps show before "Show N more". */
export const IDLE_APPS_SHOWN = 6;

/** "Last 3 inactive", or fewer when it has fewer. */
export function recentWords(count: number): string {
  return `Last ${Math.min(count, RECENT_INLINE)} inactive`;
}
