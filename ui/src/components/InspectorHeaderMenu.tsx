/**
 * AJ-E1537 — the ⋯ in a page's header: the same menu as the item's rows
 * (the action registry), less "Open its page" (you're on it).
 */
import type { JSX } from "react";
import { useItemMenu } from "./ItemMenu";
import { sessionTitle, useProjectManage, useSessionManage } from "./ManageWork";
import { showToast } from "./Toast";
import { ON_ITS_PAGE, type ActionHost, type ItemKind, type ItemOf } from "../lib/actions";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
import { toolSpec } from "../lib/tools";
import { useAccountManage } from "../screens/AccountManage";

function HeaderMenu<K extends ItemKind>({ kind, item, heading, local }: { kind: K; item: ItemOf[K]; heading: string; local: ActionHost }): JSX.Element {
  const menu = useItemMenu(kind, item, heading, { ...local, ...ON_ITS_PAGE });
  return (
    <span className="contents" {...menu.triggerProps}>
      {menu.button({ label: `Actions for ${heading}`, shown: true, testId: "header-menu" })}
      {menu.element}
    </span>
  );
}

function SessionMenu({ session, onChanged }: { session: InstanceInfo; onChanged: () => void }): JSX.Element {
  const manage = useSessionManage(onChanged);
  return (
    <>
      <HeaderMenu kind="session" item={session} heading={sessionTitle(session)} local={manage.host} />
      {manage.element}
    </>
  );
}

function ProjectMenu({ project, onChanged }: { project: ProjectInfo; onChanged: () => void }): JSX.Element {
  const manage = useProjectManage(onChanged);
  return (
    <>
      <HeaderMenu kind="project" item={project} heading={project.name} local={manage.host} />
      {manage.element}
    </>
  );
}

function AccountMenu({ account, sessions, onChanged }: { account: AccountInfo; sessions: InstanceInfo[]; onChanged: () => void }): JSX.Element {
  const manage = useAccountManage({
    sessions,
    changed: (words) => {
      if (words) showToast({ id: "account-changed", message: words });
      onChanged();
    },
    failed: (message) => showToast({ id: "account-changed", message }),
  });
  return (
    <>
      <HeaderMenu kind="account" item={account} heading={account.label} local={manage.host} />
      {manage.element}
    </>
  );
}

/** The header's menu for whatever the page shows (`kind`, as the
 * inspector names it). */
export function InspectorHeaderMenu({
  kind,
  data,
  sessions,
  onChanged,
}: {
  kind: string;
  data: { instance?: InstanceInfo | null; project?: ProjectInfo | null; account?: AccountInfo | null; provider?: { id: string } | null };
  sessions: InstanceInfo[];
  onChanged: () => void;
}): JSX.Element | null {
  if (kind === "instance" && data.instance) return <SessionMenu session={data.instance} onChanged={onChanged} />;
  if (kind === "project" && data.project) return <ProjectMenu project={data.project} onChanged={onChanged} />;
  if (kind === "account" && data.account) return <AccountMenu account={data.account} sessions={sessions} onChanged={onChanged} />;
  if (kind === "provider" && data.provider) {
    const tool = toolSpec(data.provider.id);
    return <HeaderMenu kind="tool" item={tool} heading={tool.display_name} local={{}} />;
  }
  return null;
}
