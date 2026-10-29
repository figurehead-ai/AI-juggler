/**
 * AJ-E1537, AJ-E2106 — the action registry's shapes (`lib/actions.ts`
 * reads them; the per-type declarations live in `action-specs*.ts`).
 */
import type { AccountInfo, InstanceInfo, ProjectInfo } from "./bridge";
import type { ShortcutId } from "./shortcuts";
import type { ActionHost } from "./action-host";
import type { ToolSpecInfo } from "../types/generated/ToolSpecInfo";
import type { ToolTreeInfo } from "../types/generated/ToolTreeInfo";
import type { ConversationInfo } from "../types/generated/ConversationInfo";

/** The things that have a menu, and what each one is. */
export interface ItemOf {
  session: InstanceInfo;
  project: ProjectInfo;
  account: AccountInfo;
  tool: ToolSpecInfo;
  history: InstanceInfo;
  "this-mac": ToolTreeInfo;
  /** AJ-E1529: a conversation AI Juggler didn't start. */
  conversation: ConversationInfo;
}
export type ItemKind = keyof ItemOf;

/** AJ-E2106: a menu's three groups, in this order. */
export type ActionGroup = "go" | "work" | "stop";
export const GROUP_ORDER: readonly ActionGroup[] = ["go", "work", "stop"];
/** What each group is, in words (the ⌘K panel and the evidence). */
export const GROUP_WORDS: Record<ActionGroup, string> = { go: "Getting there", work: "Working on it", stop: "Stopping" };

/** AJ-E2106: the one icon set (drawn by `components/ActionIcon`). */
export const ACTION_ICON_NAMES = [
  "go-to", "page", "project", "carry-over", "carry-in", "rename", "snooze", "mute", "wake", "copy", "pin", "stop", "new",
  "editor", "finder", "mark", "compare", "account", "sign-in", "settings", "default", "replay", "archive", "relocate",
  "remove", "delete", "info", "pool",
] as const;
export type ActionIconName = (typeof ACTION_ICON_NAMES)[number];

/** The submenus (one level, never nested), each with its label and icon. */
export const SUBMENUS = {
  snooze: { label: "Snooze or mute", icon: "snooze" },
} as const satisfies Record<string, { label: string; icon: ActionIconName }>;
export type SubmenuId = keyof typeof SUBMENUS;

/** A fixed value, or one worked out from the item and the host. */
export type Of<K extends ItemKind, T> = T | ((item: ItemOf[K], host: ActionHost) => T);

export interface ActionSpec<K extends ItemKind> {
  id: string;
  label: Of<K, string>;
  icon: Of<K, ActionIconName>;
  group: ActionGroup;
  /** Its place in its group: lower is used more, and shows first. */
  rank: number;
  /** In this submenu, not on the menu itself. */
  submenu?: SubmenuId;
  /** Its shortcut, from the shortcut registry (AJ-E1504). */
  shortcut?: ShortcutId;
  /** Whether it shows at all: its capability is there and it can ever
   * apply to this item. Never possible here → left out. */
  applies: (item: ItemOf[K], host: ActionHost) => boolean;
  /** Why it can't run now; `null` when it can. Not possible right now →
   * greyed, with this reason. */
  blocked?: (item: ItemOf[K], host: ActionHost) => string | null;
  run: (item: ItemOf[K], host: ActionHost) => void | Promise<void>;
}

/** One menu line, ready for any trigger. */
export interface ResolvedAction {
  id: string;
  label: string;
  icon: ActionIconName;
  group: ActionGroup;
  rank: number;
  submenu?: SubmenuId;
  /** Stopping or removing: last, in red. */
  destructive: boolean;
  /** The shortcut's keys, as the ? sheet shows them. */
  shortcut?: string;
  /** The shortcut itself (a row's ⌘↩ runs the action that has it). */
  shortcutId?: ShortcutId;
  /** Why it's disabled; absent when it's enabled. */
  reason?: string;
  run: () => void | Promise<void>;
}

/** One line of a menu: an action, or a submenu holding some. */
export type MenuEntry =
  | { kind: "action"; action: ResolvedAction; group: ActionGroup }
  | { kind: "submenu"; id: SubmenuId; label: string; icon: ActionIconName; group: ActionGroup; actions: ResolvedAction[] };
