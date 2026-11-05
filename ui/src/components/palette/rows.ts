/**
 * AJ-E2901: ⌘K's rows and their groups. With nothing typed: the selected
 * item's actions, Recent, the top Actions (New session…, Settings, and
 * Processes or Notifications when they have something), then the
 * sessions open now (the go-to hotkey's reason to exist). Typed: Selected
 * · Actions · Sessions · Apps · Projects · Accounts · Settings · Search,
 * each ranked by the loose match and capped at 5 ("4 more"). Pure: the
 * palette hands in what it has.
 */
import type { AccountInfo, InstanceInfo, ProjectInfo, RemoteInstanceInfo, RemoteMachineInfo } from "../../lib/bridge";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import type { ResolvedAction } from "../../lib/actions";
import type { AppModel } from "../../lib/apps";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { commandsFor, type AppDoings, type PaletteCommand } from "../../lib/palette-commands";
import { rank, type Range } from "../../lib/palette-match";
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
  const local = rank(found.sessions, text, (row) => ({ title: sessionTitle(row), other: itemWords(sessionItem(row, () => null, false)), recency: row.last_activity_ms }));
  const elsewhere = rank(found.conversations, text, (conversation) => ({
    title: conversationTitle(conversation),
    other: itemWords(conversationItem(conversation, () => null, null)),
    recency: conversation.last_activity_ms ?? 0,
  }));
  const remote = input.remoteMachines.flatMap((machine) =>
    rank(
      machine.instances.filter((row) => isLive(row.status) || row.status === "waiting_user"),
      text,
      (row) => ({ title: row.account_label, other: [row.provider, row.kind, machine.label] }),
    ).map(({ item }): PaletteRow => ({ where: "remote", machine, row: item, id: `${machine.machine_id}::${item.id}`, desktop: item.kind === "desktop" })),
  );
  const rows = [...local.map(({ item, ranges }) => sessionRow(item, ranges)), ...elsewhere.map(({ item, ranges }) => conversationRow(item, ranges)), ...remote];
  // Nothing typed: desktop windows first (the hotkey's reason to exist).
  if (text === "") return [...rows.filter((row) => "desktop" in row && row.desktop), ...rows.filter((row) => !("desktop" in row && row.desktop))];
  return rows;
}

/** Every group, in order. */
export function buildGroups(input: PaletteInputs): PaletteGroup[] {
  const selected = group("selected", input.selectedHeading ? `Actions for ${input.selectedHeading}` : "Selected", input.actions.map((action) => ({ where: "action", action, id: `action::${action.id}` })), false);
  if (!typedAnything(input.query)) {
    return [...selected, ...group("recent", "Recent", recentRows(input), false), ...group("actions", "Actions", topActions(input), false), ...group("open", "Open now", sessionRows(input), false)];
  }
  const text = input.query.text;
  const shown = input.app ? commandsFor(input.commands, text, input.app) : [];
  const commands = shown.filter((command) => command.id !== "search-history");
  const search = shown.filter((command) => command.id === "search-history");
  const apps = rank(input.apps, text, (app) => ({ title: app.name, other: [app.defaultName, app.app, app.profile ?? ""] })).map(({ item, ranges }) => appRow(item, appOpen(item, input.sessions), ranges));
  apps.sort((a, b) => Number(b.where === "app" && b.open) - Number(a.where === "app" && a.open));
  const projects = rank(input.projects, text, (project) => ({ title: project.name, other: [project.root], recency: project.last_activity_ms ?? 0 })).map(({ item, ranges }) => projectRow(item, ranges));
  const accounts = rank(input.accounts, text, (account) => ({ title: account.name, other: [account.tool, account.subscription.plan ?? ""] })).map(({ item, ranges }) => accountRow(item, ranges));
  return [
    ...selected,
    ...group("actions", "Actions", commands.map(commandRow)),
    ...group("sessions", "Sessions", sessionRows(input)),
    ...(text ? group("apps", "Apps", apps) : []),
    ...(text ? group("projects", "Projects", projects) : []),
    ...(text ? group("accounts", "Accounts", accounts) : []),
    ...(text ? group("settings", "Settings", input.settings.map((hit): PaletteRow => ({ where: "setting", hit, id: `setting::${hit.pane}::${hit.anchor}` })), true).map((entry) => ({ ...entry, rows: entry.rows.slice(0, 3), more: 0 })) : []),
    ...group("search", "Search", search.map(commandRow), false),
  ];
}
