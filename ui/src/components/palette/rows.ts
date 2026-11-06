/**
 * AJ-E2901: ⌘K's rows and their groups. With nothing typed: the selected
 * item's actions, Recent, the top Actions (New session…, Settings, and
 * Processes or Notifications when they have something), then the
 * sessions open now (the go-to hotkey's reason to exist), less any shown
 * under Recent. Typed: Selected, then Actions · Sessions · Apps · Projects
 * · Accounts · Settings ordered by how well each group's best row matches
 * (an exact "payments-api" above a loose "Go to Privacy"; on a tie, that
 * order), then Search; each ranked by the loose match and capped at 5 ("4
 * more"). Pure: the palette hands in what it has.
 */
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { RemoteInstanceInfo } from "../../types/generated/RemoteInstanceInfo";
import type { RemoteMachineInfo } from "../../types/generated/RemoteMachineInfo";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import type { ResolvedAction } from "../../lib/actions";
import type { AppModel } from "../../lib/apps";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { commandsFor, type AppDoings, type PaletteCommand } from "../../lib/palette-commands";
import { fuzzyMatch, rank, type Range } from "../../lib/palette-match";
import { conversationItem, sessionItem } from "../../lib/rail-items";
import { itemWords, paletteMatches, type RailQuery } from "../../lib/rail-view";
import type { Recent } from "../../lib/palette-recents";
import type { SearchHit } from "../../screens/settings/SettingsSearch";
import { sessionTitle } from "../ManageWork";
import { conversationTitle } from "../../lib/conversations";

/** An Account as ⌘K lists it: its name, its tool, its room in words, and
 * the folder a new session on it starts in. */
export interface AccountRow {
  key: string;
  accountId: string | null;
  name: string;
  tool: string;
  room: string;
  folder: AccountInfo | null;
  subscription: SubscriptionInfo;
}

export type PaletteRow =
  | { where: "action"; action: ResolvedAction; id: string }
  | { where: "command"; command: PaletteCommand; id: string }
  | { where: "project"; project: ProjectInfo; id: string; ranges: Range[] }
  | { where: "local"; row: InstanceInfo; id: string; desktop: boolean; ranges: Range[] }
  | { where: "outside"; conversation: ConversationInfo; id: string; ranges: Range[] }
  | { where: "remote"; machine: RemoteMachineInfo; row: RemoteInstanceInfo; id: string; desktop: boolean }
  | { where: "app"; app: AppModel; id: string; open: boolean; ranges: Range[] }
  | { where: "account"; account: AccountRow; id: string; ranges: Range[] }
  | { where: "setting"; hit: SearchHit; id: string };

export interface PaletteGroup {
  key: string;
  label: string;
  rows: PaletteRow[];
  /** How many more matched than are shown. */
  more: number;
}

export interface PaletteInputs {
  /** What's typed, as typed. */
  typed: string;
  query: RailQuery;
  actions: ResolvedAction[];
  selectedHeading: string | null;
  commands: PaletteCommand[];
  app?: AppDoings;
  sessions: InstanceInfo[];
  conversations: ConversationInfo[];
  remoteMachines: RemoteMachineInfo[];
  projects: ProjectInfo[];
  apps: AppModel[];
  accounts: AccountRow[];
  recents: Recent[];
  leftovers: number;
  unread: number;
  settings: SearchHit[];
}

export const CAP = 5;
const isLive = (status: string) => status === "running" || status === "starting";

/** Is anything typed (words, or the rail's `is:` and `app:` words)? */
export function typedAnything(query: RailQuery): boolean {
  return query.text !== "" || query.is.size > 0 || Object.keys(query.only).length > 0;
}

/** Whether an App runs now (a desktop app's process, or a session of a CLI one). */
export function appOpen(app: AppModel, sessions: InstanceInfo[]): boolean {
  if (app.place) return app.place.app_pid !== null;
  return app.folder !== null && sessions.some((session) => session.account_id === app.folder!.id && isLive(session.status));
}

const sessionRow = (row: InstanceInfo, ranges: Range[] = []): PaletteRow => ({ where: "local", row, id: row.id, desktop: row.kind === "desktop", ranges });
const conversationRow = (conversation: ConversationInfo, ranges: Range[] = []): PaletteRow => ({ where: "outside", conversation, id: `outside::${conversation.account_id}::${conversation.id}`, ranges });
const appRow = (app: AppModel, open: boolean, ranges: Range[] = []): PaletteRow => ({ where: "app", app, id: `app::${app.id}`, open, ranges });
const projectRow = (project: ProjectInfo, ranges: Range[] = []): PaletteRow => ({ where: "project", project, id: `project::${project.id}`, ranges });
const accountRow = (account: AccountRow, ranges: Range[] = []): PaletteRow => ({ where: "account", account, id: `account::${account.key}`, ranges });
const commandRow = (command: PaletteCommand): PaletteRow => ({ where: "command", command, id: `command::${command.id}` });
const remoteRow = (machine: RemoteMachineInfo, row: RemoteInstanceInfo): PaletteRow => ({ where: "remote", machine, row, id: `${machine.machine_id}::${row.id}`, desktop: row.kind === "desktop" });

/** What a row is matched by: its title, its other words, its recency
 * (none for the selected item's actions, which aren't matched). */
function fieldsOf(row: PaletteRow): { title: string; other: readonly string[]; recency?: number | null } | null {
  switch (row.where) {
    case "action":
      return null;
    case "command":
      // A screen's command is named by its screen ("usage" names Go to
      // Usage as surely as a setting called "Usage statistics…").
      return { title: row.command.id.startsWith("go:") ? row.command.label.replace(/^Go to /, "") : row.command.label, other: [row.command.words ?? ""] };
    case "project":
      return { title: row.project.name, other: [row.project.root], recency: row.project.last_activity_ms ?? 0 };
    case "local":
      return { title: sessionTitle(row.row), other: itemWords(sessionItem(row.row, () => null, false)), recency: row.row.last_activity_ms };
    case "outside":
      return { title: conversationTitle(row.conversation), other: itemWords(conversationItem(row.conversation, () => null, null)), recency: row.conversation.last_activity_ms ?? 0 };
    case "remote":
      return { title: row.row.account_label, other: [row.row.provider, row.row.kind, row.machine.label] };
    case "app":
      return { title: row.app.name, other: [row.app.defaultName, row.app.app, row.app.profile ?? ""] };
    case "account":
      return { title: row.account.name, other: [row.account.tool, row.account.subscription.plan ?? ""] };
    case "setting":
      return { title: row.hit.title, other: [row.hit.paneLabel, row.hit.words] };
  }
}

/** `rows` that match `text`, best first, each with its title's matched letters. */
function ranked(rows: PaletteRow[], text: string): PaletteRow[] {
  return rank(rows, text, (row) => fieldsOf(row) ?? { title: "" }).map(({ item, ranges }) => ("ranges" in item ? { ...item, ranges } : item));
}

/** How well a row matches `text` (0: not at all, or not matched). */
function scoreOf(row: PaletteRow, text: string): number {
  const fields = fieldsOf(row);
  return fields ? (fuzzyMatch(text, fields.title, fields.other)?.score ?? 0) : 0;
}

/** The recent ones still here, as rows. */
export function recentRows(input: PaletteInputs): PaletteRow[] {
  return input.recents.flatMap((recent): PaletteRow[] => {
    switch (recent.kind) {
      case "session": {
        const row = input.sessions.find((session) => session.id === recent.id);
        return row ? [sessionRow(row)] : [];
      }
      case "conversation": {
        const found = input.conversations.find((conversation) => `${conversation.account_id}::${conversation.id}` === recent.id);
        return found ? [conversationRow(found)] : [];
      }
      case "app": {
        const app = input.apps.find((entry) => entry.id === recent.id);
        return app ? [appRow(app, appOpen(app, input.sessions))] : [];
      }
      case "project": {
        const project = input.projects.find((entry) => entry.id === recent.id);
        return project ? [projectRow(project)] : [];
      }
      case "account": {
        const account = input.accounts.find((entry) => entry.key === recent.id);
        return account ? [accountRow(account)] : [];
      }
      case "command": {
        const command = input.commands.find((entry) => entry.id === recent.id);
        return command ? [commandRow(command)] : [];
      }
    }
  });
}

/** The Recent kind and id a row is remembered by (none for actions on
 * the selected item, a search, a setting or another computer's session). */
export function recentOf(row: PaletteRow): Recent | null {
  switch (row.where) {
    case "local":
      return { kind: "session", id: row.row.id };
    case "outside":
      return { kind: "conversation", id: `${row.conversation.account_id}::${row.conversation.id}` };
    case "app":
      return { kind: "app", id: row.app.id };
    case "project":
      return { kind: "project", id: row.project.id };
    case "account":
      return { kind: "account", id: row.account.key };
    case "command":
      return row.command.id === "search-history" ? null : { kind: "command", id: row.command.id };
    default:
      return null;
  }
}

/** The top actions before anything is typed. */
function topActions(input: PaletteInputs): PaletteRow[] {
  const byId = (id: string) => input.commands.find((command) => command.id === id);
  const rows: PaletteCommand[] = [];
  const add = (command: PaletteCommand | undefined, label?: string) => command && rows.push(label ? { ...command, label } : command);
  add(byId("new-session"));
  add(byId("go:settings"), "Settings");
  if (input.leftovers > 0) add(byId("go:cleanup"), `Processes — ${input.leftovers} left running`);
  if (input.unread > 0) add(byId("notifications"), `Notifications — ${input.unread} unread`);
  return rows.map(commandRow);
}

const group = (key: string, label: string, rows: PaletteRow[], cap = true): PaletteGroup[] =>
  rows.length === 0 ? [] : [{ key, label, rows: cap ? rows.slice(0, CAP) : rows, more: cap ? Math.max(0, rows.length - CAP) : 0 }];

/** The sessions for the query: AI Juggler's, other apps', other computers'. */
function sessionRows(input: PaletteInputs): PaletteRow[] {
  const found = paletteMatches(input.sessions, input.conversations, input.projects, input.query);
  const text = input.query.text;
  const local = found.sessions.map((row) => sessionRow(row));
  const elsewhere = found.conversations.map((conversation) => conversationRow(conversation));
  const remote = input.remoteMachines.flatMap((machine) => machine.instances.filter((row) => isLive(row.status) || row.status === "waiting_user").map((row) => remoteRow(machine, row)));
  // Typed: the best match first, wherever it runs.
  if (text !== "") return ranked([...local, ...elsewhere, ...remote], text);
  // Nothing typed: desktop windows first (the hotkey's reason to exist).
  const rows = [...ranked(local, text), ...ranked(elsewhere, text), ...remote];
  return [...rows.filter((row) => "desktop" in row && row.desktop), ...rows.filter((row) => !("desktop" in row && row.desktop))];
}

/** Every group, in order. */
export function buildGroups(input: PaletteInputs): PaletteGroup[] {
  const selected = group("selected", input.selectedHeading ? `Actions for ${input.selectedHeading}` : "Selected", input.actions.map((action) => ({ where: "action", action, id: `action::${action.id}` })), false);
  if (!typedAnything(input.query)) {
    // Once each: a session under Recent isn't listed again under Open now.
    const recent = recentRows(input);
    const inRecent = new Set(recent.map((row) => row.id));
    return [...selected, ...group("recent", "Recent", recent, false), ...group("actions", "Actions", topActions(input), false), ...group("open", "Open now", sessionRows(input).filter((row) => !inRecent.has(row.id)), false)];
  }
  const text = input.query.text;
  const shown = input.app ? commandsFor(input.commands, text, input.app) : [];
  const commands = shown.filter((command) => command.id !== "search-history").map(commandRow);
  const search = shown.filter((command) => command.id === "search-history");
  const apps = ranked(input.apps.map((app) => appRow(app, appOpen(app, input.sessions))), text);
  apps.sort((a, b) => Number(b.where === "app" && b.open) - Number(a.where === "app" && a.open));
  const projects = ranked(input.projects.map((project) => projectRow(project)), text);
  const accounts = ranked(input.accounts.map((account) => accountRow(account)), text);
  const sessions = sessionRows(input);
  const settings = input.settings.map((hit): PaletteRow => ({ where: "setting", hit, id: `setting::${hit.pane}::${hit.anchor}` }));
  // Each group scored by its best row (before the cap); on a tie the
  // order here stands, so an action group leads only when it matches at
  // least as well as the objects (the 0.4 design review, 16).
  const scored = (entries: PaletteGroup[], rows: PaletteRow[]) => entries.map((entry) => ({ entry, score: Math.max(0, ...rows.map((row) => scoreOf(row, text))) }));
  const matched = [
    ...scored(group("actions", "Actions", commands), commands),
    ...scored(group("sessions", "Sessions", sessions), sessions),
    ...(text
      ? [
          ...scored(group("apps", "Apps", apps), apps),
          ...scored(group("projects", "Projects", projects), projects),
          ...scored(group("accounts", "Accounts", accounts), accounts),
          ...scored(group("settings", "Settings", settings).map((entry) => ({ ...entry, rows: entry.rows.slice(0, 3), more: 0 })), settings),
        ]
      : []),
  ].sort((a, b) => b.score - a.score);
  return [...selected, ...matched.map(({ entry }) => entry), ...group("search", "Search", search.map(commandRow), false)];
}
