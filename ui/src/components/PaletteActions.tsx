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

/** What is selected, as the registry knows it. */
export type PaletteSelection = { [K in ItemKind]: { kind: K; item: ItemOf[K]; heading: string } }[ItemKind];

/** App's selection (a session, project, account or tool) for ⌘K. */
export function paletteSelection(
  session: InstanceInfo | null,
  project: ProjectInfo | null,
  account: AccountInfo | null,
  toolId: string | null,
): PaletteSelection | null {
  if (session) return { kind: "session", item: session, heading: sessionTitle(session) };
  if (project) return { kind: "project", item: project, heading: project.name };
  if (account) return { kind: "account", item: account, heading: account.label };
  if (toolId) {
    const tool = toolSpec(toolId);
    return { kind: "tool", item: tool, heading: tool.display_name };
  }
  return null;
}

/** The selected item's actions that match what's typed, and the sheets
 * they open (renaming, a project's mark…), which outlive the palette. */
export function usePaletteActions(selected: PaletteSelection | null, needle: string, onChanged: () => void): { actions: ResolvedAction[]; element: ReactNode } {
  const shared = useActionHost();
  const sessions = useSessionManage(onChanged);
  const projects = useProjectManage(onChanged);
  const host = { ...shared, ...sessions.host, ...projects.host, ...ON_ITS_PAGE };
  const all = selected ? resolveActions(selected.kind, selected.item as never, host) : [];
  const actions = needle ? all.filter((action) => fullLabel(action).toLowerCase().includes(needle)) : all;
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
      <ActionIcon name={action.icon} className={action.reason ? "text-faint" : action.destructive ? "text-danger" : "text-muted"} />
      <span className={"min-w-0 shrink truncate " + (action.reason ? "text-faint" : action.destructive ? "text-danger" : "")}>{fullLabel(action)}</span>
      {action.reason && <span className="min-w-0 shrink truncate text-[10.5px] text-faint">{action.reason}</span>}
      {action.shortcut && <kbd className="ml-auto shrink-0 font-mono text-[10px] text-faint">{action.shortcut}</kbd>}
    </>
  );
}
