/**
 * A member's menu on the canvas (⋯, right-click, Shift+F10): the row
 * menus' one pattern (D-25) — open it, make it the lead or an observer,
 * stop it, take it out of the pool (last, apart). A greyed line says why.
 */
import type { JSX } from "react";

import { MenuItem, MenuSeparator, RowMenu } from "../../components/RowMenu";
import { useActionHost } from "../../components/ItemMenu";
import { IconPage, IconRemove, IconStop } from "../../components/icons-actions";
import { IconLead } from "../../components/icons-canvas";
import { IconRead } from "../../components/icons-menu";
import type { PoolInfo } from "../../lib/ipc/pools";
import { cannotRemove, makeLead, nameOf, setRole } from "./actions";
import type { MemberNode } from "./useCanvas";

const LIVE = new Set(["starting", "running", "waiting_user", "stuck"]);

function Line({ icon, children }: { icon: JSX.Element; children: string }): JSX.Element {
  return (
    <span className="flex items-center gap-2">
      <span className="flex size-4 shrink-0 items-center justify-center text-muted" aria-hidden="true">
        {icon}
      </span>
      {children}
    </span>
  );
}

export function NodeMenu({
  pool,
  member,
  members,
  at,
  onClose,
  onRemove,
}: {
  pool: PoolInfo;
  member: MemberNode;
  members: MemberNode[];
  at: { x: number; y: number };
  onClose: () => void;
  onRemove: (member: MemberNode) => void;
}): JSX.Element {
  const host = useActionHost();
  const removeWhy = cannotRemove(pool);
  const live = LIVE.has(member.session.status);
  const run = (work: () => unknown) => () => {
    onClose();
    void work();
  };
  return (
    <RowMenu heading={nameOf(member.session)} ariaLabel={`${nameOf(member.session)} actions`} at={at} onClose={onClose}>
      <MenuItem onClick={run(() => host.openSessionById?.(member.id))} disabled={!host.openSessionById} testId="node-menu-open">
        <Line icon={<IconPage width={15} height={15} />}>Open session</Line>
      </MenuItem>
      {member.role === "lead" ? (
        <MenuItem onClick={run(() => setRole(member, "member"))} testId="node-menu-unlead">
          <Line icon={<IconLead width={15} height={15} />}>Make it a member</Line>
        </MenuItem>
      ) : (
        <MenuItem onClick={run(() => makeLead(member, members))} testId="node-menu-lead">
          <Line icon={<IconLead width={15} height={15} />}>Make lead</Line>
        </MenuItem>
      )}
      <MenuItem onClick={run(() => setRole(member, member.role === "observer" ? "member" : "observer"))} testId="node-menu-observer">
        <Line icon={<IconRead width={15} height={15} />}>{member.role === "observer" ? "Let it work here again" : "Only let it watch"}</Line>
      </MenuItem>
      {live && host.stopSession && (
        <MenuItem onClick={run(() => host.stopSession?.(member.session))} testId="node-menu-stop">
          <Line icon={<IconStop width={15} height={15} />}>Stop session</Line>
        </MenuItem>
      )}
      <MenuSeparator />
      <MenuItem onClick={run(() => onRemove(member))} disabled={removeWhy !== null} danger describedBy={removeWhy ? "node-menu-remove-why" : undefined} testId="node-menu-remove">
        <Line icon={<IconRemove width={15} height={15} />}>Take out of the pool</Line>
      </MenuItem>
      {removeWhy && (
        <span id="node-menu-remove-why" className="px-2 pb-1 text-label text-muted">
          {removeWhy}
        </span>
      )}
    </RowMenu>
  );
}
