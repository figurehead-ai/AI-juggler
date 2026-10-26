/**
 * AJ-E1537 — one action registry (ADR-0019). Every item type declares its
 * actions once (`action-specs*.ts`): an id, a label, its icon, its group
 * and rank, when it applies, when it's enabled (and why not), a
 * shortcut, and what it does. The right-click menu, the ⋯ menu, the
 * keyboard menu and ⌘K all read the same list, in the same order.
 *
 * AJ-E2106 — menus read in one glance (Apple's menu guidance): at most
 * three groups, getting there first, working on it next, stopping and
 * removing last (in red); most used first within each group (`rank`);
 * one level of submenus. Every action has one icon, the same wherever it
 * shows. What shows: an action that can never apply to this item is
 * left out (`applies`); one that can't run right now is greyed with the
 * reason (`blocked`).
 *
 * An action does its work through the screen's {@link ActionHost}: the
 * screens that own a capability (App, the rail, Accounts, History)
 * provide it, and an action whose capability isn't there doesn't show.
 * So no screen keeps a menu of its own, and no action is written twice.
 */
import { SHORTCUTS } from "./shortcuts";
import type { ActionHost } from "./action-host";
import { CONVERSATION, HISTORY, SESSION } from "./action-specs";
import { ACCOUNT, PROJECT, THIS_MAC, TOOL } from "./action-specs-setup";
import { GROUP_ORDER, SUBMENUS, type ActionSpec, type ItemKind, type ItemOf, type MenuEntry, type Of, type ResolvedAction } from "./action-types";

export type { ActionHost };
export { isLive } from "./action-specs";
export { ACTION_ICON_NAMES, GROUP_ORDER, GROUP_WORDS, SUBMENUS } from "./action-types";
export type { ActionGroup, ActionIconName, ActionSpec, ItemKind, ItemOf, MenuEntry, ResolvedAction, SubmenuId } from "./action-types";

const REGISTRY: { [K in ItemKind]: ActionSpec<K>[] } = {
  session: SESSION,
  project: PROJECT,
  account: ACCOUNT,
  tool: TOOL,
  history: HISTORY,
  "this-mac": THIS_MAC,
  conversation: CONVERSATION,
};

export const ITEM_KINDS = Object.keys(REGISTRY) as ItemKind[];

/** The registry's declarations for one type (tests read them). */
export function actionSpecs<K extends ItemKind>(kind: K): readonly ActionSpec<K>[] {
  return REGISTRY[kind];
}

/** On an item's own page (its header's ⋯, ⌘K there): no "Open its page". */
export const ON_ITS_PAGE: ActionHost = {
  openSession: undefined,
  openProject: undefined,
  openAccount: undefined,
  openTool: undefined,
  openConversation: undefined,
};

const value = <K extends ItemKind, T>(of: Of<K, T>, item: ItemOf[K], host: ActionHost): T =>
  typeof of === "function" ? (of as (item: ItemOf[K], host: ActionHost) => T)(item, host) : of;

/** The menu for `item`: the actions that apply, getting there first and
 * stopping last, most used first in each group, each with its label,
 * icon and, when it can't run, why. A disabled action's `run` does
 * nothing. */
export function resolveActions<K extends ItemKind>(kind: K, item: ItemOf[K], host: ActionHost): ResolvedAction[] {
  const specs = REGISTRY[kind] as ActionSpec<K>[];
  return GROUP_ORDER.flatMap((group) =>
    specs
      .filter((spec) => spec.group === group && spec.applies(item, host))
      .sort((a, b) => a.rank - b.rank)
      .map((spec): ResolvedAction => {
        const reason = spec.blocked?.(item, host) ?? undefined;
        return {
          id: spec.id,
          label: value(spec.label, item, host),
          icon: value(spec.icon, item, host),
          group,
          rank: spec.rank,
          submenu: spec.submenu,
          destructive: group === "stop",
          shortcut: spec.shortcut ? SHORTCUTS.find((entry) => entry.id === spec.shortcut)?.keys[0] : undefined,
          shortcutId: spec.shortcut,
          reason: reason ?? undefined,
          run: reason ? () => {} : () => spec.run(item, host),
        };
      }),
  );
}

/** The resolved actions as a menu shows them: each submenu once, where
 * its first action ranks, holding its actions (one level, never more). */
export function menuEntries(actions: ResolvedAction[]): MenuEntry[] {
  const entries: MenuEntry[] = [];
  const open = new Map<string, Extract<MenuEntry, { kind: "submenu" }>>();
  for (const action of actions) {
    if (!action.submenu) {
      entries.push({ kind: "action", action, group: action.group });
      continue;
    }
    const existing = open.get(action.submenu);
    if (existing) {
      existing.actions.push(action);
      continue;
    }
    const entry = { kind: "submenu" as const, id: action.submenu, ...SUBMENUS[action.submenu], group: action.group, actions: [action] };
    open.set(action.submenu, entry);
    entries.push(entry);
  }
  return entries;
}

/** How a submenu's action reads out of its submenu (⌘K lists them flat). */
export function fullLabel(action: ResolvedAction): string {
  if (!action.submenu) return action.label;
  return `${SUBMENUS[action.submenu].label}: ${action.label.charAt(0).toLowerCase()}${action.label.slice(1)}`;
}
