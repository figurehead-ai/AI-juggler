/**
 * AJ-E2106 — ⌘K's action panel: with an item selected, its actions come
 * first, from the same registry as its menus (the same icons, order and
 * greyed reasons, each with its shortcut), then the go-to results. On
 * the item's page already, so no "Open its page".
 */
import type { JSX, ReactNode } from "react";
import { ActionIcon } from "./ActionIcon";
import { useActionHost } from "./ItemMenu";
import { sessionTitle, useProjectManage, useSessionManage } from "./ManageWork";
import { ON_ITS_PAGE, fullLabel, resolveActions, type ItemKind, type ItemOf, type ResolvedAction } from "../lib/actions";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
import { toolSpec } from "../lib/tools";
import { conversationTitle } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { PaletteCommand, PaletteCommandIcon } from "../lib/palette-commands";
import { IconFleet, IconProfiles, IconProjects, IconShield, IconUsage } from "./icons";
import { IconHistory, IconView } from "./icons-places";
import { IconBell, IconBroom, IconComputer, IconPage, IconPlus } from "./icons-actions";
import { IconGear, IconKeyboard } from "./icons-menu";
import type { SVGProps } from "react";

/** What is selected, as the registry knows it. */
export type PaletteSelection = { [K in ItemKind]: { kind: K; item: ItemOf[K]; heading: string } }[ItemKind];

/** App's selection (a session, a session another app runs, a project,
 * an account or a tool) for ⌘K. */
export function paletteSelection(
  session: InstanceInfo | null,
  project: ProjectInfo | null,
  account: AccountInfo | null,
  toolId: string | null,
  conversation: ConversationInfo | null = null,
): PaletteSelection | null {
  if (session) return { kind: "session", item: session, heading: sessionTitle(session) };
  // The 0.2 walk (F7): a session another app runs had no actions here.
  if (conversation) return { kind: "conversation", item: conversation, heading: conversationTitle(conversation) };
  if (project) return { kind: "project", item: project, heading: project.name };
  if (account) return { kind: "account", item: account, heading: account.label };
  if (toolId) {
    const tool = toolSpec(toolId);
    return { kind: "tool", item: tool, heading: tool.display_name };
  }
  return null;
}

/** Other words an action is found by in ⌘K (the 0.2 walk, F7: "switch"
 * found nothing; the action's name stays D-05's). */
const ACTION_WORDS: Record<string, string> = {
  continue: "switch hand over another account move",
  focus: "go to window front bring",
  "go-to": "window front bring",
  stop: "kill end quit",
  "copy-resume": "command terminal",
};

/** The selected item's actions that match what's typed, and the sheets
 * they open (renaming, a project's mark…), which outlive the palette. */
export function usePaletteActions(selected: PaletteSelection | null, needle: string, onChanged: () => void): { actions: ResolvedAction[]; element: ReactNode } {
  const shared = useActionHost();
  const sessions = useSessionManage(onChanged);
  const projects = useProjectManage(onChanged);
  const host = { ...shared, ...sessions.host, ...projects.host, ...ON_ITS_PAGE };
  const all = selected ? resolveActions(selected.kind, selected.item as never, host) : [];
  const actions = needle ? all.filter((action) => `${fullLabel(action)} ${ACTION_WORDS[action.id] ?? ""}`.toLowerCase().includes(needle)) : all;
  return {
    actions,
    element: (
      <>
        {sessions.element}
        {projects.element}
      </>
    ),
  };
}

/** One action as a palette option: icon, label, why it's greyed, keys. */
export function ActionOption({ action }: { action: ResolvedAction }): JSX.Element {
  return (
    <>
      <ActionIcon name={action.icon} className={action.reason ? "text-muted" : action.destructive ? "text-danger" : "text-muted"} />
      <span className={"min-w-0 shrink truncate " + (action.reason ? "text-muted" : action.destructive ? "text-danger" : "")}>{fullLabel(action)}</span>
      {action.reason && <span className="min-w-0 shrink truncate text-label text-muted">{action.reason}</span>}
      {action.shortcut && <kbd className="ml-auto shrink-0 font-mono text-label text-muted">{action.shortcut}</kbd>}
    </>
  );
}

/** App's actions' icons: the ones the same screens and controls show
 * elsewhere (your menu, the header). Decoration only. */
const COMMAND_ICONS: Record<PaletteCommandIcon, (props: SVGProps<SVGSVGElement>) => JSX.Element> = {
  home: IconPage,
  screen: IconPage,
  accounts: IconProfiles,
  usage: IconUsage,
  history: IconHistory,
  processes: IconBroom,
  computer: IconComputer,
  settings: IconGear,
  privacy: IconShield,
  fleet: IconFleet,
  new: IconPlus,
  project: IconProjects,
  bell: IconBell,
  keyboard: IconKeyboard,
  list: IconView,
  search: IconHistory,
};

/** One of App's actions as a palette option: icon, label, keys. */
export function CommandOption({ command }: { command: PaletteCommand }): JSX.Element {
  const Icon = COMMAND_ICONS[command.icon];
  return (
    <>
      <Icon width={14} height={14} aria-hidden="true" focusable="false" className="shrink-0 text-muted" data-icon={command.icon} />
      <span className="min-w-0 shrink truncate">{command.label}</span>
      {command.keys && <kbd className="ml-auto shrink-0 font-mono text-label text-muted">{command.keys}</kbd>}
    </>
  );
}
