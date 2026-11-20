/**
 * SK-13: a named session's role, one control wherever the session shows:
 * its inspector and its row in the List view (the canvas's twin, for the
 * keyboard and screen readers), as its card's ⋯ on the canvas sets it.
 * Lead makes it the pool's one lead (the lead it had becomes a member).
 */
import { useState, type JSX } from "react";

import type { PoolInfo, PoolRole } from "../../lib/ipc/pools";
import { asRole } from "../../lib/pool-words";
import { makeLead, setRole, type RoleTarget } from "./actions";
import { RolePicker } from "./setup/RoleCards";
import type { MemberNode } from "./useCanvas";

export function MemberRole({ pool, member, members, prefix = "Its role:", testId = "member-role" }: { pool: PoolInfo; member: RoleTarget; members: readonly Pick<MemberNode, "id" | "role" | "session">[]; prefix?: string; testId?: string }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const change = (role: PoolRole) => {
    if (role === asRole(member.role)) return;
    setBusy(true);
    void (role === "lead" ? makeLead(pool, member, members) : setRole(pool, member, role)).finally(() => setBusy(false));
  };
  return <RolePicker member={`session:${member.id}`} name={member.name} value={asRole(member.role)} prefix={prefix} disabled={busy} onChange={change} testId={testId} />;
}
