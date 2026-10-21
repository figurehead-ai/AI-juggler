/**
 * AJ-E1537 — one action registry (ADR-0019). Every item type declares its
 * actions once, here: an id, a label, its group, when it applies, when
 * it's enabled (and why not), whether it's destructive, a shortcut, and
 * what it does. The right-click menu, the ⋯ menu and the keyboard menu
 * all read the same list, in the same order.
 *
 * An action does its work through the screen's {@link ActionHost}: the
 * screens that own a capability (App, the rail, Accounts, History)
 * provide it, and an action whose capability isn't there doesn't show.
 * So no screen keeps a menu of its own, and no action is written twice.
 */
import type { AccountInfo, InstanceInfo, ProjectInfo } from "./bridge";
import { SHORTCUTS, type ShortcutId } from "./shortcuts";
import type { ToolSpecInfo } from "../types/generated/ToolSpecInfo";
import type { ToolTreeInfo } from "../types/generated/ToolTreeInfo";

/** The things that have a menu, and what each one is. */
export interface ItemOf {
  session: InstanceInfo;
  project: ProjectInfo;
  account: AccountInfo;
  tool: ToolSpecInfo;
  history: InstanceInfo;
  "this-mac": ToolTreeInfo;
}
export type ItemKind = keyof ItemOf;

/** What the screens can do, by item type. Each piece is optional: the
 * screen that owns it provides it (an `ActionHostProvider`). */
export interface ActionHost {
  // Sessions
  openSession?: (row: InstanceInfo) => void;
  focusSession?: (row: InstanceInfo) => void | Promise<void>;
  continueSession?: (row: InstanceInfo) => void;
  renameSession?: (row: InstanceInfo) => void;
  pinSession?: (row: InstanceInfo) => void | Promise<void>;
  copyResumeCommand?: (row: InstanceInfo) => void | Promise<void>;
  revealSessionProject?: (row: InstanceInfo) => void;
  stopSession?: (row: InstanceInfo) => void;
  /** The rail's attention controls (snooze, mute). */
  snoozeSession?: (row: InstanceInfo, ms: number | null) => void;
  muteSession?: (row: InstanceInfo) => void;
  unsnoozeSession?: (row: InstanceInfo) => void;
  isSnoozed?: (row: InstanceInfo) => boolean;
  // Projects
  openProject?: (project: ProjectInfo) => void;
  newSessionIn?: (project: ProjectInfo) => void;
  openProjectIn?: (project: ProjectInfo, how: "editor" | "finder") => void | Promise<void>;
  /** The editor projects open in, when one is chosen (Settings). */
  editorName?: () => string | null;
  copyProjectPath?: (project: ProjectInfo) => void | Promise<void>;
  renameProject?: (project: ProjectInfo) => void;
  pinProject?: (project: ProjectInfo) => void | Promise<void>;
  relocateProject?: (project: ProjectInfo) => void | Promise<void>;
  forgetProject?: (project: ProjectInfo) => void;
  setProjectAccount?: (project: ProjectInfo) => void;
  // Accounts
  openAccount?: (account: AccountInfo) => void;
  startSessionOn?: (account: AccountInfo) => void;
  /** Why a session can't start on it now (`null`: it can). */
  startBlockedWhy?: (account: AccountInfo) => string | null;
  signIn?: (account: AccountInfo) => void;
  editLaunchSettings?: (account: AccountInfo) => void;
  renameAccount?: (account: AccountInfo) => void;
  toggleDefaultAccount?: (account: AccountInfo) => void;
  removeAccount?: (account: AccountInfo) => void;
  // Tools
  openTool?: (tool: ToolSpecInfo) => void;
  copyUpdateCommand?: (tool: ToolSpecInfo) => void | Promise<void>;
  toolAccounts?: (tool: ToolSpecInfo) => void;
  // History
  openHistory?: (row: InstanceInfo) => void;
  replay?: (row: InstanceInfo) => void;
  continueHistory?: (row: InstanceInfo) => void;
  archive?: (row: InstanceInfo) => void | Promise<void>;
  unarchive?: (row: InstanceInfo) => void | Promise<void>;
  isArchived?: (row: InstanceInfo) => boolean;
  deleteHistory?: (row: InstanceInfo) => void;
  // This Mac
  openSessionById?: (id: string) => void;
}

export type ActionGroup = "main" | "attention" | "destructive";

export interface ActionSpec<K extends ItemKind> {
  id: string;
  label: string | ((item: ItemOf[K], host: ActionHost) => string);
  group: ActionGroup;
  /** Its shortcut, from the shortcut registry (AJ-E1504). */
  shortcut?: ShortcutId;
  /** Whether it shows at all: its capability is there and it applies. */
  applies: (item: ItemOf[K], host: ActionHost) => boolean;
  /** Why it can't run now; `null` when it can. */
  blocked?: (item: ItemOf[K], host: ActionHost) => string | null;
  run: (item: ItemOf[K], host: ActionHost) => void | Promise<void>;
}

/** One menu line, ready for any trigger. */
export interface ResolvedAction {
  id: string;
  label: string;
  group: ActionGroup;
  destructive: boolean;
  /** The shortcut's keys, as the ? sheet shows them. */
  shortcut?: string;
  /** Why it's disabled; absent when it's enabled. */
  reason?: string;
  run: () => void | Promise<void>;
}

const GROUP_ORDER: ActionGroup[] = ["main", "attention", "destructive"];

/** The statuses of a session that is still going. */
const LIVE = new Set(["starting", "running", "waiting_user", "stuck", "quota_blocked"]);
export const isLive = (row: InstanceInfo): boolean => LIVE.has(row.status);

const has = (fn: unknown): boolean => typeof fn === "function";

const SESSION: ActionSpec<"session">[] = [
  { id: "open", label: "Open", group: "main", applies: (_, h) => has(h.openSession), run: (row, h) => h.openSession!(row) },
  {
    id: "focus",
    label: (row) => (row.kind === "desktop" ? "Focus its window" : "Show its terminal"),
    group: "main",
    applies: (row, h) => has(h.focusSession) && isLive(row),
    run: (row, h) => h.focusSession!(row),
  },
  {
    id: "continue",
    label: "Continue on another account…",
    group: "main",
    shortcut: "handover",
    applies: (_, h) => has(h.continueSession),
    blocked: (row) => (row.conversation_id ? null : "It has no conversation to carry over yet"),
    run: (row, h) => h.continueSession!(row),
  },
  { id: "rename", label: "Rename…", group: "main", applies: (_, h) => has(h.renameSession), run: (row, h) => h.renameSession!(row) },
  {
    id: "pin",
    label: (row) => (row.pinned ? "Unpin" : "Pin to top"),
    group: "main",
    applies: (_, h) => has(h.pinSession),
    run: (row, h) => h.pinSession!(row),
  },
  {
    id: "copy-resume",
    label: "Copy resume command",
    group: "main",
    applies: (_, h) => has(h.copyResumeCommand),
    blocked: (row) => (row.conversation_id ? null : "It has no conversation to resume yet"),
    run: (row, h) => h.copyResumeCommand!(row),
  },
  {
    id: "reveal-project",
    label: "Show its project",
    group: "main",
    applies: (row, h) => has(h.revealSessionProject) && row.project_id !== null,
    run: (row, h) => h.revealSessionProject!(row),
  },
  {
    id: "snooze-15",
    label: "Snooze 15m",
    group: "attention",
    applies: (row, h) => has(h.snoozeSession) && !h.isSnoozed?.(row),
    run: (row, h) => h.snoozeSession!(row, 15 * 60_000),
  },
  {
    id: "snooze-60",
    label: "Snooze 1h",
    group: "attention",
    applies: (row, h) => has(h.snoozeSession) && !h.isSnoozed?.(row),
    run: (row, h) => h.snoozeSession!(row, 60 * 60_000),
  },
  {
    id: "snooze-reset",
    label: "Until reset",
    group: "attention",
    applies: (row, h) => has(h.snoozeSession) && !h.isSnoozed?.(row),
    blocked: (row) => (row.quota?.resets_at_ms != null ? null : "No known reset time for this quota window"),
    run: (row, h) => h.snoozeSession!(row, null),
  },
  {
    id: "mute",
    label: "Mute this session",
    group: "attention",
    applies: (row, h) => has(h.muteSession) && !h.isSnoozed?.(row),
    run: (row, h) => h.muteSession!(row),
  },
  {
    id: "unsnooze",
    label: "Unsnooze / unmute",
    group: "attention",
    applies: (row, h) => has(h.unsnoozeSession) && h.isSnoozed?.(row) === true,
    run: (row, h) => h.unsnoozeSession!(row),
  },
  {
    id: "stop",
    label: "Stop",
    group: "destructive",
    shortcut: "stop",
    applies: (row, h) => has(h.stopSession) && isLive(row),
    run: (row, h) => h.stopSession!(row),
  },
];

const PROJECT: ActionSpec<"project">[] = [
  { id: "open", label: "Open", group: "main", applies: (_, h) => has(h.openProject), run: (p, h) => h.openProject!(p) },
  {
    id: "new-session",
    label: "New session here",
    group: "main",
    applies: (_, h) => has(h.newSessionIn),
    blocked: (p) => (p.missing ? "Its folder isn't there any more" : null),
    run: (p, h) => h.newSessionIn!(p),
  },
  {
    id: "open-editor",
    label: (_, h) => (h.editorName?.() ? `Open in ${h.editorName()}` : "Open in Finder"),
    group: "main",
    applies: (_, h) => has(h.openProjectIn),
    blocked: (p) => (p.missing ? "Its folder isn't there any more" : null),
    run: (p, h) => h.openProjectIn!(p, "editor"),
  },
  {
    id: "reveal",
    label: "Show in Finder",
    group: "main",
    applies: (_, h) => has(h.openProjectIn) && !!h.editorName?.(),
    blocked: (p) => (p.missing ? "Its folder isn't there any more" : null),
    run: (p, h) => h.openProjectIn!(p, "finder"),
  },
  { id: "copy-path", label: "Copy path", group: "main", applies: (_, h) => has(h.copyProjectPath), run: (p, h) => h.copyProjectPath!(p) },
  { id: "rename", label: "Rename…", group: "main", applies: (_, h) => has(h.renameProject), run: (p, h) => h.renameProject!(p) },
  {
    id: "pin",
    label: (p) => (p.pinned ? "Unpin" : "Pin to top"),
    group: "main",
    applies: (_, h) => has(h.pinProject),
    run: (p, h) => h.pinProject!(p),
  },
  {
    id: "default-account",
    label: "Set its account…",
    group: "main",
    applies: (_, h) => has(h.setProjectAccount),
    run: (p, h) => h.setProjectAccount!(p),
  },
  {
    id: "relocate",
    label: "Relocate…",
    group: "main",
    applies: (p, h) => has(h.relocateProject) && p.missing === true,
    run: (p, h) => h.relocateProject!(p),
  },
  { id: "forget", label: "Forget…", group: "destructive", applies: (_, h) => has(h.forgetProject), run: (p, h) => h.forgetProject!(p) },
];

const ACCOUNT: ActionSpec<"account">[] = [
  { id: "open", label: "Open", group: "main", applies: (_, h) => has(h.openAccount), run: (a, h) => h.openAccount!(a) },
  {
    id: "start",
    label: "Start a session",
    group: "main",
    applies: (_, h) => has(h.startSessionOn),
    blocked: (a, h) => h.startBlockedWhy?.(a) ?? null,
    run: (a, h) => h.startSessionOn!(a),
  },
  { id: "sign-in", label: "Sign in…", group: "main", applies: (_, h) => has(h.signIn), run: (a, h) => h.signIn!(a) },
  {
    id: "launch-settings",
    label: "Edit launch settings",
    group: "main",
    applies: (_, h) => has(h.editLaunchSettings),
    run: (a, h) => h.editLaunchSettings!(a),
  },
  { id: "rename", label: "Rename…", group: "main", applies: (_, h) => has(h.renameAccount), run: (a, h) => h.renameAccount!(a) },
  {
    id: "default",
    label: (a) => (a.is_default ? "Stop being the default" : "Make the default"),
    group: "main",
    applies: (_, h) => has(h.toggleDefaultAccount),
    run: (a, h) => h.toggleDefaultAccount!(a),
  },
  { id: "remove", label: "Remove…", group: "destructive", applies: (_, h) => has(h.removeAccount), run: (a, h) => h.removeAccount!(a) },
];

const TOOL: ActionSpec<"tool">[] = [
  { id: "open", label: "Open", group: "main", applies: (_, h) => has(h.openTool), run: (t, h) => h.openTool!(t) },
  {
    id: "copy-update",
    label: "Copy update command",
    group: "main",
    applies: (t, h) => has(h.copyUpdateCommand) && t.install_command !== null,
    run: (t, h) => h.copyUpdateCommand!(t),
  },
  { id: "accounts", label: "Its accounts", group: "main", applies: (_, h) => has(h.toolAccounts), run: (t, h) => h.toolAccounts!(t) },
];

const HISTORY: ActionSpec<"history">[] = [
  { id: "open", label: "Open", group: "main", applies: (_, h) => has(h.openHistory), run: (row, h) => h.openHistory!(row) },
  { id: "replay", label: "Replay", group: "main", applies: (_, h) => has(h.replay), run: (row, h) => h.replay!(row) },
  {
    id: "continue",
    label: "Continue on another account…",
    group: "main",
    applies: (_, h) => has(h.continueHistory),
    blocked: (row) => (row.conversation_id ? null : "It has no conversation to carry over"),
    run: (row, h) => h.continueHistory!(row),
  },
  {
    id: "archive",
    label: (row, h) => (h.isArchived?.(row) ? "Unarchive" : "Archive"),
    group: "main",
    applies: (_, h) => has(h.archive) && has(h.unarchive),
    run: (row, h) => (h.isArchived?.(row) ? h.unarchive!(row) : h.archive!(row)),
  },
  { id: "delete", label: "Delete…", group: "destructive", applies: (_, h) => has(h.deleteHistory), run: (row, h) => h.deleteHistory!(row) },
];

const THIS_MAC: ActionSpec<"this-mac">[] = [
  {
    id: "open",
    label: "Go to the session",
    group: "main",
    // A tool AI Juggler didn't start has no actions (D-08).
    applies: (tree, h) => has(h.openSessionById) && tree.session_id !== null,
    run: (tree, h) => h.openSessionById!(tree.session_id!),
  },
  {
    id: "not-ours",
    label: "Not started by AI Juggler",
    group: "main",
    applies: (tree) => tree.session_id === null,
    blocked: () => "AI Juggler only acts on sessions it started",
    run: () => {},
  },
];

const REGISTRY: { [K in ItemKind]: ActionSpec<K>[] } = {
  session: SESSION,
  project: PROJECT,
  account: ACCOUNT,
  tool: TOOL,
  history: HISTORY,
  "this-mac": THIS_MAC,
};

/** The registry's declarations for one type (tests read them). */
export function actionSpecs<K extends ItemKind>(kind: K): readonly ActionSpec<K>[] {
  return REGISTRY[kind];
}

/** The menu for `item`: the actions that apply, main first and the
 * destructive ones last, each with its label and, when it can't run,
 * why. A disabled action's `run` does nothing. */
export function resolveActions<K extends ItemKind>(kind: K, item: ItemOf[K], host: ActionHost): ResolvedAction[] {
  const specs = REGISTRY[kind] as ActionSpec<K>[];
  return GROUP_ORDER.flatMap((group) =>
    specs
      .filter((spec) => spec.group === group && spec.applies(item, host))
      .map((spec): ResolvedAction => {
        const reason = spec.blocked?.(item, host) ?? undefined;
        return {
          id: spec.id,
          label: typeof spec.label === "function" ? spec.label(item, host) : spec.label,
          group,
          destructive: group === "destructive",
          shortcut: spec.shortcut ? SHORTCUTS.find((entry) => entry.id === spec.shortcut)?.keys[0] : undefined,
          reason: reason ?? undefined,
          run: reason ? () => {} : () => spec.run(item, host),
        };
      }),
  );
}
