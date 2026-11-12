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
import type { AppModel } from "./apps";
import type { PlaceOfferInfo } from "../types/generated/PlaceOfferInfo";
import { appName, cliAppKey, cliAppWord, desktopAppKey, desktopAppWord, desktopProfile } from "./app-name";
import { tools } from "./tools";
import { openNow, type RailItem } from "./rail-items";
import type { RailGroup } from "./rail-places";

/** D-100: what Inactive apps needs of a CLI App of the Apps list. */
export type CliAppFacts = Pick<AppModel, "railKey" | "name" | "app" | "profile" | "appId" | "tool"> & { folder: { id: string } | null };

/** One app with nothing open. */
export interface IdleApp {
  /** The App's key, as the rail's groups key it. */
  key: string;
  kind: "desktop" | "cli";
  /** "Claude app · b-east", "Claude Code · Work Max". */
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
  /** 0.4.3: its own folder, desktop or CLI: its Account's Out or low mark. */
  folder: string | null;
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
    for (const key of place.recent) {
      // A place whose folder is known claims only its own folder's copy
      // (the backend's `app_of`): a switch copies a conversation into
      // another profile's folder under the same id.
      const at = conversationAt(place.account_id, key);
      if (!out.has(at)) out.set(at, knownKey(place));
    }
  }
  return out;
}

/** The key [`appByConversation`] keeps a conversation (`<tool>/<id>`) under:
 * with the folder its files are in, when the place knows its folder. */
function conversationAt(folder: string | null, key: string): string {
  return folder ? `${folder}|${key}` : key;
}

/** The App key [`appByConversation`] gives conversation `key` in `folder`. */
export function appOfConversation(byConversation: ReadonlyMap<string, string>, folder: string, key: string): string | undefined {
  return byConversation.get(conversationAt(folder, key)) ?? byConversation.get(key);
}

/** Each remembered desktop app's App key by its id (`@<id>`: where a
 * session AI Juggler launched in it belongs), and by the folder its
 * sessions run on when it's the one profile there (`|<folder>`). rc22
 * (A3): never "the most recently seen" of two on one folder: that
 * guessed, and the backend (`places::app_of`) counted elsewhere. */
export function appByFolder(known: readonly KnownPlaceInfo[]): Map<string, string> {
  const out = new Map<string, string>();
  const linked = new Map<string, KnownPlaceInfo[]>();
  for (const place of known) {
    out.set(`@${place.id}`, knownKey(place));
    if (place.account_id === null) continue;
    linked.set(place.account_id, [...(linked.get(place.account_id) ?? []), place]);
  }
  // 0.4.1: a folder only one desktop profile runs on (`|<folder>`): its
  // sessions are that profile's, closed or not (a closed Claude app
  // profile's Code sessions fell to the folder's CLI App, with no Open).
  // ADR-0032: shared, the one that's its app's default instance, when one
  // is (the backend's `sole_holders`).
  for (const [folder, places] of linked) {
    const keys = [...new Set(places.map(knownKey))];
    const defaults = [...new Set(places.filter((place) => place.instance === null).map(knownKey))];
    const one = keys.length === 1 ? keys[0] : defaults.length === 1 ? defaults[0] : undefined;
    if (one) out.set(`|${folder}`, one);
  }
  return out;
}

/** The App an item belongs to, in the backend's order (`places::app_of`,
 * `fixtures/session-apps.json`): an open one where it's open; AI
 * Juggler's desktop session, the profile it was launched in; another
 * app's, the profile whose records hold it; one its tool says a desktop
 * app started, the one profile on its folder; else its folder's CLI App.
 * ADR-0032: a conversation the backend gave an App (`app`) is that App's,
 * first and always. */
export function appKeyOf(item: RailItem, byConversation: ReadonlyMap<string, string>, byFolder: ReadonlyMap<string, string> = new Map()): string {
  const conversation = item.conversation;
  // ADR-0032 (D-100): the backend says whose it is (its process while it's
  // open, where it was last seen when it isn't); nothing here guesses.
  const owner = conversation?.app;
  if (conversation && owner) {
    if (owner.startsWith("app:")) return byFolder.get(`@${owner}`) ?? item.place.key;
    return item.place.key.startsWith("cli|") ? item.place.key : cliAppKey(conversation.tool, conversation.account_id, conversation.account_label);
  }
  if (conversation && !openNow(item)) {
    const desktop = appOfConversation(byConversation, conversation.account_id, `${conversation.tool}/${conversation.id}`);
    if (desktop) return desktop;
    // Its tool says a desktop app started it: not a terminal's.
    const started = conversation.started_in ? byFolder.get(`|${conversation.account_id}`) : undefined;
    if (started) return started;
  }
  // A desktop session AI Juggler started: the profile it was launched in,
  // else the one profile on its folder, else its folder's CLI App.
  const own = item.session;
  if (own && item.place.kind === "own" && item.place.key.startsWith("app|") && item.accountId) {
    return (own.app_id ? byFolder.get(`@${own.app_id}`) : undefined) ?? byFolder.get(`|${item.accountId}`) ?? cliAppKey(own.provider, item.accountId, own.account_label);
  }
  return item.place.key;
}

/** Each App's sessions that aren't open (archived ones left out), newest
 * first. */
export function idleSessionsByApp(items: readonly RailItem[], byConversation: ReadonlyMap<string, string>, byFolder: ReadonlyMap<string, string> = new Map()): Map<string, RailItem[]> {
  const out = new Map<string, RailItem[]>();
  for (const item of items) {
    if (openNow(item) || item.archived || item.need !== null) continue;
    const key = appKeyOf(item, byConversation, byFolder);
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
  cliApps = [],
}: {
  openKeys: ReadonlySet<string>;
  known: readonly KnownPlaceInfo[];
  idle: ReadonlyMap<string, RailItem[]>;
  /** D-100: the Apps list's CLI Apps, so Inactive apps is the same set as
   * Accounts' Apps that aren't running (one whose sessions the window
   * hasn't read, older ones, too). */
  cliApps?: readonly CliAppFacts[];
}): IdleApp[] {
  const out: IdleApp[] = [];
  const desktop = new Set<string>();
  for (const place of known) {
    const key = knownKey(place);
    desktop.add(key);
    if (openKeys.has(key)) continue;
    const sessions = idle.get(key) ?? [];
    const app = knownAppWord(place);
    const profile = desktopProfile(place.instance, place.profile);
    out.push({
      key,
      kind: "desktop",
      name: appName(app, profile),
      app,
      profile,
      appId: place.app_id,
      // AJ-E3203: one only found by its folder was never seen running:
      // no "just now", and it goes after the ones you've used.
      lastMs: Math.max(sessions[0]?.lastMs ?? 0, place.found ? 0 : place.seen_ms),
      appPid: place.app_pid,
      placeId: place.id,
      cannotStart: place.cannot_start,
      accountId: null,
      folder: place.account_id,
      tool: null,
      sessions,
    });
  }
  for (const [key, sessions] of idle) {
    const first = sessions[0];
    if (!first || openKeys.has(key) || desktop.has(key) || !key.startsWith("cli|")) continue;
    const tool = first.conversation?.tool ?? first.session?.provider ?? null;
    const app = tool ? cliAppWord(tool) : first.place.app;
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
      folder: first.accountId,
      tool,
      sessions,
    });
  }
  for (const app of cliApps) {
    if (openKeys.has(app.railKey) || out.some((one) => one.key === app.railKey)) continue;
    out.push({ key: app.railKey, kind: "cli", name: app.name, app: app.app, profile: app.profile, appId: app.appId, lastMs: 0, appPid: null, placeId: null, cannotStart: null, accountId: app.folder?.id ?? null, folder: app.folder?.id ?? null, tool: app.tool, sessions: [] });
  }
  return out.sort((a, b) => b.lastMs - a.lastMs || a.name.localeCompare(b.name));
}

/** How many inactive sessions the rail shows inline, under an app. */
export const RECENT_INLINE = 3;
/** How many inactive apps show before "Show N more". */
export const IDLE_APPS_SHOWN = 6;

/** "Last 3 inactive" (D-64), or fewer when it has fewer; one is "Last
 * inactive", not "Last 1 inactive" (design review #70). */
export function recentWords(count: number): string {
  const shown = Math.min(count, RECENT_INLINE);
  return shown === 1 ? "Last inactive" : `Last ${shown} inactive`;
}

/** ADR-0032 (D-100): the groups of the Apps that run with nothing open in
 * them (none of `have`), so Open now is exactly the Apps that run; an App
 * kept off the rail (`off`) isn't one. 0.4.3: each carries its App's own
 * folder (`account_id`), so its header marks its Account out or low with
 * nothing open in it. */
export function runningGroups(known: readonly KnownPlaceInfo[], have: ReadonlySet<string>, off: (key: string) => boolean): RailGroup[] {
  const out: RailGroup[] = [];
  for (const place of known) {
    const key = knownKey(place);
    if (place.app_pid === null || have.has(key) || off(key) || out.some((group) => group.key === key)) continue;
    const app = knownAppWord(place);
    out.push({ key, label: app, place: { key, app, appId: place.app_id, kind: "desktop", profile: desktopProfile(place.instance, place.profile), appPid: place.app_pid, host: null }, accountId: place.account_id, poolId: null, items: [] });
  }
  return out;
}

/** 0.4.3: the folder an App's Out or low mark is read from: its own, never
 * a session's (a session in it may run on another Account's folder, and an
 * App with nothing open has no session to ask). A desktop App's is its
 * remembered folder; a CLI App's is in its key (`cli|<tool>|<folder>`);
 * else, as before, its first session's. */
export function appFolderOf(group: Pick<RailGroup, "key" | "items">, known: Pick<KnownPlaceInfo, "account_id"> | null | undefined): string | null {
  if (known?.account_id) return known.account_id;
  const [kind, , folder] = group.key.split("|");
  if (kind === "cli" && folder) return folder;
  return group.items[0]?.accountId ?? null;
}

/** ADR-0032 Amendment 1 (D-84, D-100): the rail's "Running, not added"
 * section, between Open now and Inactive apps: its head, then one line
 * per app an adapter recognises running that the user hasn't added. */
export type OfferLine = { kind: "section"; id: "offered"; count: number } | { kind: "offer"; offer: PlaceOfferInfo };

/** An offer's App id (`app:<app>:<folder>`, `cli:<folder>`) as the App
 * the user has would carry it: a desktop App's id, a CLI App's
 * `cli:<folder>` from its rail key (`cli|<tool>|<folder>`). */
function idOfRailKey(key: string): string | null {
  const [kind, , folder] = key.split("|");
  return kind === "cli" && folder ? `cli:${folder}` : null;
}

/** "Running, not added": each running offer once, by its id, unless the
 * user has it already (a known App's id, or a CLI App in Open now or
 * Inactive apps by its folder): an App is in exactly one of Open now,
 * Running not added and Inactive apps. None, no section. */
export function offeredLines(offers: readonly PlaceOfferInfo[], known: readonly KnownPlaceInfo[], shownKeys: Iterable<string>): OfferLine[] {
  const taken = new Set<string>(known.map((place) => place.id));
  for (const key of shownKeys) {
    const id = idOfRailKey(key);
    if (id) taken.add(id);
  }
  const seen = new Set<string>();
  const running = offers.filter((offer) => offer.running && !taken.has(offer.id) && !seen.has(offer.id) && Boolean(seen.add(offer.id)));
  if (running.length === 0) return [];
  return [{ kind: "section", id: "offered", count: running.length }, ...running.map((offer) => ({ kind: "offer" as const, offer }))];
}
