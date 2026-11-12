/**
 * The rail matrix's kit (D-64, D-100, ADR-0032 with its amendments 1, 2
 * and 8): an invented world of Apps and sessions, one runner that builds
 * the rail's lines as the place board does (`itemsOf`, then `buildLines`),
 * a reader that turns the lines into what a person sees (each App's
 * section, what sits under it, its links), and the checks every row runs.
 *
 * Expectations come from the spec, not the code: who owns a session is the
 * backend's `ConversationInfo.app` (ADR-0032 §1), mapped here by the
 * fixture's own table, never by the rail's `appKeyOf`.
 */
import { expect } from "vitest";
import { buildLines, type Line, type Lines, type SectionId } from "./rail-lines";
import { itemsOf } from "./rail-items";
import { recentWords, type CliAppFacts } from "./rail-apps";
import { DEFAULT_VIEW, parseQuery, type RailView } from "./rail-view";
import type { AccountOf } from "./rail-places";
import type { OrderMode } from "./rail-order";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { KnownPlaceInfo } from "./ipc/places";
import type { PlaceOfferInfo } from "../types/generated/PlaceOfferInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

export const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();
export const H = 3_600_000;

/** One App of the world: its rail key, the backend's id for it (what a
 * session's `app` names), its folder (its Account's), and, for a desktop
 * App, what AI Juggler remembers of it. */
export interface FixtureApp {
  key: string;
  id: string;
  folder: string;
  label: string;
  tool: string;
  desktop: KnownPlaceInfo | null;
  place: PlaceInfo;
}

const TERMINAL: PlaceInfo = { app: "Terminal", shown: "Terminal", app_id: null, profile: null, instance: null, pid: 0, kind: "terminal", app_pid: 900, label: "Terminal" };

/** A Claude app profile ("Claude app · East"), running (`pid`) or not. */
export function claudeApp(name: string, pid: number | null): FixtureApp {
  const folder = `f-${name.toLowerCase()}`;
  const desktop: KnownPlaceInfo = {
    id: `app:claude-app:/p/${name}`, app_id: "claude-app", app: "Claude", instance: name, profile: name, label: `Claude · ${name}`, app_pid: pid, lost: false,
    keep_alive: false, paused: null, account_id: folder, seen_ms: NOW - 5 * H, cannot_start: null, recent: [], found: false,
  };
  const place: PlaceInfo = { app: "Claude", shown: "Claude app", app_id: "claude-app", profile: name, instance: name, pid: 0, kind: "desktop", app_pid: pid, label: `Claude · ${name}` };
  return { key: `app|claude-app|${name}`, id: desktop.id, folder, label: name, tool: "claude-code", desktop, place };
}

/** The Codex app in ChatGPT.app (`app:codex-app:`, ADR-0032 §1), on `~/.codex`. */
export function codexApp(pid: number | null): FixtureApp {
  const desktop: KnownPlaceInfo = {
    id: "app:codex-app:", app_id: "codex-app", app: "ChatGPT", instance: null, profile: null, label: "Codex", app_pid: pid, lost: false,
    keep_alive: false, paused: null, account_id: "f-codex", seen_ms: NOW - 5 * H, cannot_start: null, recent: [], found: false,
  };
  const place: PlaceInfo = { app: "ChatGPT", shown: "Codex", app_id: "codex-app", profile: null, instance: null, pid: 0, kind: "desktop", app_pid: pid, label: "Codex" };
  return { key: "app|codex-app|", id: desktop.id, folder: "f-codex", label: "Codex home", tool: "codex", desktop, place };
}

/** A CLI App: `tool` in a terminal on `folder` (`cli:<folder>`). */
export function cliApp(tool: string, folder: string, label: string): FixtureApp {
  return { key: `cli|${tool}|${folder}`, id: `cli:${folder}`, folder, label, tool, desktop: null, place: TERMINAL };
}

/** A session of `app` open now. */
export function open(id: string, app: FixtureApp, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: app.tool, account_id: app.folder, account_label: app.label, project_id: null, cwd: "/w/app", title: `Talk ${id}`, git_branch: "feature",
    started_ms: NOW - 2 * H, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [{ ...app.place, pid: 500 }], app: app.id,
    pid: 500, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
  };
}

/** A session of `app` that isn't open, last active `hours` ago. */
export function past(id: string, app: FixtureApp, hours: number, over: Partial<ConversationInfo> = {}): ConversationInfo {
  const last = NOW - hours * H;
  return open(id, app, { live: false, places: [], pid: null, state: null, last_activity_ms: last, started_ms: last - H, ...over });
}

/** Several past sessions, `<prefix>1` newest. */
export function pasts(prefix: string, app: FixtureApp, count: number): ConversationInfo[] {
  return Array.from({ length: count }, (_, n) => past(`${prefix}${n + 1}`, app, n + 1));
}

const project = (id: string, name: string): ProjectInfo => ({
  id, root: `/w/${name}`, name, last_activity_ms: NOW, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false,
});
export const PROJECTS: ProjectInfo[] = [project("p-api", "api"), project("p-web", "web")];

/** The subscriptions: two Claude profiles' folders on one Account. */
const ACCOUNTS: Record<string, { key: string; label: string }> = {
  "f-east": { key: "acct-team", label: "Team" },
  "f-west": { key: "acct-team", label: "Team" },
  "f-north": { key: "acct-team", label: "Team" },
  "f-south": { key: "acct-solo", label: "Solo" },
  "f-cli": { key: "acct-solo", label: "Solo" },
  "f-codex": { key: "acct-codex", label: "Codex Plus" },
};
const ACCOUNT_OF: AccountOf = (folder) => (folder === null ? null : (ACCOUNTS[folder] ?? null));

interface Expectation {
  /** Each section's keys (Apps, Accounts or offers), any order. */
  open?: string[];
  offered?: string[];
  idle?: string[];
  /** Section heads' counts; `null`: no such head. */
  counts?: Partial<Record<SectionId, number | null>>;
  /** What sits under each key, top to bottom (session ids). */
  under?: Record<string, string[]>;
  /** The link lines under each key ("Last 3 inactive", "Show all", "+2 more"). */
  links?: Record<string, string[]>;
  foldable?: Record<string, boolean>;
  /** The headers drawn folded. */
  folded?: string[];
  /** Each header's `group.accountId`: whose Out mark it wears. */
  folder?: Record<string, string | null>;
  depth?: Record<string, number>;
  /** "Show n more" under Inactive apps. */
  idleMore?: number;
  also?: Partial<{ ended: number; inactive: number }>;
}

export interface MatrixRow {
  name: string;
  lens: "place" | "account";
  query?: string;
  only?: RailView["only"];
  /** The user's Apps (a CLI App also by its sessions). */
  apps: FixtureApp[];
  /** CLI Apps the user added (the Apps list's). */
  added?: FixtureApp[];
  talks: ConversationInfo[];
  /** Session ids the rail pinned. */
  pins?: string[];
  offers?: PlaceOfferInfo[];
  toggled?: string[];
  order?: { mode?: OrderMode; folded?: string[] };
  expect: Expectation;
  /** Where the code disagrees with the spec: "<spec> vs <code>" (the row runs as `it.fails`). */
  mismatch?: string;
}

const cliFacts = (app: FixtureApp): CliAppFacts => ({ railKey: app.key, name: `${app.tool} CLI · ${app.label}`, app: `${app.tool} CLI`, profile: app.label, appId: app.tool, tool: app.tool, folder: { id: app.folder } });

/** The rail's lines for a row, as the place board builds them. */
function run(row: MatrixRow): Lines {
  const pins = new Set((row.pins ?? []).map((id) => keyOf(row.talks.find((talk) => talk.id === id)!)));
  const items = itemsOf({ sessions: [], conversations: row.talks, projects: PROJECTS, seen: {}, pins });
  const view: RailView = { ...DEFAULT_VIEW, group: row.lens, only: row.only ?? {} };
  const order = { mode: row.order?.mode ?? "yours", rows: new Map<string, number>(), groups: [], folded: new Set(row.order?.folded ?? []) };
  const known = row.apps.flatMap((app) => (app.desktop ? [app.desktop] : []));
  return buildLines({
    items, view, query: parseQuery(row.query ?? ""), now: NOW, toggled: new Set(row.toggled ?? []), frozen: null,
    accountOf: ACCOUNT_OF, order, known, offers: row.offers ?? [], cliApps: (row.added ?? []).map(cliFacts),
  });
}

const keyOf = (talk: ConversationInfo) => `c:${talk.account_id}/${talk.id}`;

/** What a person reads off the lines. */
interface Seen {
  sections: Map<SectionId, string[]>;
  counts: Map<SectionId, number>;
  under: Map<string, string[]>;
  links: Map<string, string[]>;
  headers: Map<string, Extract<Line, { kind: "header" }>>;
  depth: Map<string, number>;
  /** Each session drawn under a key: that key (Needs you's shortcut aside). */
  drawnUnder: Map<string, string[]>;
  idleMore: number;
}

interface Walk {
  seen: Seen;
  section: SectionId | null;
  at: string | null;
}

const push = (map: Map<string, string[]>, key: string, value: string) => map.set(key, [...(map.get(key) ?? []), value]);
const idOfItem = (item: { conversation: ConversationInfo | null; session: { id: string } | null }) => item.conversation?.id ?? item.session?.id ?? "?";

function drawn(walk: Walk, item: Parameters<typeof idOfItem>[0], depth: number | undefined): void {
  const id = idOfItem(item);
  walk.seen.depth.set(id, depth ?? 0);
  if (walk.at === null) return;
  push(walk.seen.under, walk.at, id);
  push(walk.seen.drawnUnder, id, walk.at);
}

/** A link line ("Last 3 inactive", "Show all", "+2 more") under the App or Account above it. */
function link(walk: Walk, words: string): void {
  if (walk.at !== null) push(walk.seen.links, walk.at, words);
}

/** Where a line puts the reader: a head starts its section, a header or an
 * Inactive apps row the App (or Account) the next lines sit under. */
function enter(line: Line, walk: Walk): void {
  switch (line.kind) {
    case "section":
      walk.section = line.id;
      walk.at = null;
      walk.seen.counts.set(line.id, line.count);
      walk.seen.sections.set(line.id, []);
      return;
    case "header":
      walk.at = line.group.key;
      walk.seen.headers.set(line.group.key, line);
      if (walk.section) push(walk.seen.sections, walk.section, line.group.key);
      return;
    case "idle":
      walk.at = line.app.key;
      push(walk.seen.sections, "idle", line.app.key);
      return;
    case "offer":
      walk.at = null;
      push(walk.seen.sections, "offered", line.offer.id);
      return;
    case "also":
      walk.section = null;
      walk.at = null;
      return;
    default:
      return;
  }
}

/** What a line shows under the App or Account above it. */
function under(line: Line, walk: Walk): void {
  switch (line.kind) {
    case "row":
    case "past":
      drawn(walk, line.item, line.depth);
      return;
    case "recent":
      link(walk, recentWords(line.count));
      return;
    case "all":
      link(walk, "Show all");
      return;
    case "more":
      link(walk, `+${line.quiet} ${line.words}`);
      return;
    case "idle-more":
      walk.seen.idleMore = line.hidden;
      return;
    default:
      return;
  }
}

function read(lines: readonly Line[]): Seen {
  const seen: Seen = { sections: new Map(), counts: new Map(), under: new Map(), links: new Map(), headers: new Map(), depth: new Map(), drawnUnder: new Map(), idleMore: 0 };
  const walk: Walk = { seen, section: null, at: null };
  for (const line of lines) {
    enter(line, walk);
    under(line, walk);
  }
  return seen;
}

/** The key a session belongs under, by the spec: its owner App (the
 * backend's `app`) in the App lens, its Account in the Account lens. */
function ownerOf(row: MatrixRow, talk: ConversationInfo): string {
  if (row.lens === "account") return ACCOUNT_OF(talk.account_id)?.key ?? talk.account_id;
  return [...row.apps, ...(row.added ?? [])].find((app) => app.id === talk.app)?.key ?? `unknown owner ${talk.app ?? ""}`;
}

const hidden = (talk: ConversationInfo) => talk.app_archived === true || talk.archived_here === true || talk.app_unlisted === true;
const LISTED: ReadonlySet<Line["kind"]> = new Set(["row", "past", "need", "plain"]);

/** No key in two sections; each session under its own App (or Account),
 * never another's; ended archived and unlisted ones never listed. */
function placement(row: MatrixRow, built: Lines, seen: Seen): void {
  const everywhere = [...seen.sections.values()].flat();
  expect(new Set(everywhere).size, "an App in two sections").toBe(everywhere.length);
  for (const talk of row.talks) for (const at of seen.drawnUnder.get(talk.id) ?? []) expect(at, `${talk.id} drawn under`).toBe(ownerOf(row, talk));
  const listed = new Set(built.lines.flatMap((line) => (LISTED.has(line.kind) && "item" in line ? [idOfItem(line.item)] : [])));
  // 0.4.3 ruling: what runs shows (D-100); archived and unlisted ones are left out once ended.
  for (const talk of row.talks.filter((one) => hidden(one) && !one.live)) expect(listed.has(talk.id), `${talk.id} (archived or unlisted) listed`).toBe(false);
}

/** A fold only when unfolding shows a row; folded, nothing under it (amendment 8). */
function folds(seen: Seen): void {
  for (const [key, header] of seen.headers) {
    const rows = (seen.under.get(key) ?? []).length + (seen.links.get(key) ?? []).length;
    if (header.folded !== null) expect(rows, `${key} folded, yet something under it`).toBe(0);
    else expect(header.foldable, `${key} foldable`).toBe((seen.under.get(key) ?? []).length > 0);
  }
}

/** Whose Out mark each header wears (UX spec §5 rule 6, amendment 8): by
 * App its own folder, never a session's; by Account a folder of that Account. */
function marks(row: MatrixRow, seen: Seen): void {
  const apps = [...row.apps, ...(row.added ?? [])];
  for (const [key, header] of seen.headers) {
    if (row.lens === "account") expect(ACCOUNT_OF(header.group.accountId)?.key, `${key}'s Out mark folder`).toBe(key);
    const app = apps.find((one) => one.key === key);
    if (row.lens === "place" && app) expect(header.group.accountId, `${key}'s Out mark folder`).toBe(app.folder);
  }
}

/** By App, the heads count the Apps in them (Inactive apps: those drawn and those behind "Show n more"). */
function heads(row: MatrixRow, seen: Seen): void {
  if (row.lens !== "place") return;
  for (const id of ["open", "idle", "offered"] as const) {
    if (!seen.counts.has(id)) continue;
    expect(seen.counts.get(id), `${id}'s count`).toBe((seen.sections.get(id) ?? []).length + (id === "idle" ? seen.idleMore : 0));
  }
}

/** What holds in every row (ADR-0032 §3, amendment 8). */
function invariants(row: MatrixRow, built: Lines, seen: Seen): void {
  placement(row, built, seen);
  folds(seen);
  marks(row, seen);
  heads(row, seen);
}

const sorted = (list: readonly string[] | undefined) => [...(list ?? [])].sort();

function sectionsMatch(want: Expectation, seen: Seen): void {
  for (const id of ["open", "offered", "idle"] as const) {
    const keys = want[id];
    if (keys !== undefined) expect(sorted(seen.sections.get(id)), `${id} holds`).toEqual(sorted(keys));
  }
  for (const id of ["needs", "open", "offered", "idle"] as const) {
    const count = want.counts?.[id];
    if (count !== undefined) expect(seen.counts.get(id) ?? null, `${id}'s head`).toBe(count);
  }
  if (want.idleMore !== undefined) expect(seen.idleMore, "Show n more (Inactive apps)").toBe(want.idleMore);
}

function underMatch(want: Expectation, seen: Seen): void {
  for (const [key, ids] of Object.entries(want.under ?? {})) expect(seen.under.get(key) ?? [], `under ${key}`).toEqual(ids);
  for (const [key, words] of Object.entries(want.links ?? {})) expect(seen.links.get(key) ?? [], `links under ${key}`).toEqual(words);
  for (const [id, depth] of Object.entries(want.depth ?? {})) expect(seen.depth.get(id), `${id}'s depth`).toBe(depth);
}

function headersMatch(want: Expectation, seen: Seen, built: Lines): void {
  for (const [key, foldable] of Object.entries(want.foldable ?? {})) expect(seen.headers.get(key)?.foldable, `${key} foldable`).toBe(foldable);
  if (want.folded) expect(sorted([...seen.headers].flatMap(([key, line]) => (line.folded !== null ? [key] : []))), "folded headers").toEqual(sorted(want.folded));
  for (const [key, folder] of Object.entries(want.folder ?? {})) expect(seen.headers.get(key)?.group.accountId, `${key}'s Out mark folder`).toBe(folder);
  if (want.also?.ended !== undefined) expect(built.also.ended, "Also: ended today").toBe(want.also.ended);
  if (want.also?.inactive !== undefined) expect(built.also.inactive, "Also: inactive").toBe(want.also.inactive);
}

/** One row of the matrix: build, read, check the invariants and the row's own expectations. */
export function check(row: MatrixRow): void {
  const built = run(row);
  const seen = read(built.lines);
  invariants(row, built, seen);
  sectionsMatch(row.expect, seen);
  underMatch(row.expect, seen);
  headersMatch(row.expect, seen, built);
}
