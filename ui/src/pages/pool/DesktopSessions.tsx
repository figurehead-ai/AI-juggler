/**
 * AJ-E3013 (f1b9bd354): the desktop sessions an App brings into a pool
 * through AI Juggler's connector, as session nodes under their App's card
 * on the diagram: each one's name, role and how a new message reaches it
 * (D-119), and a dot while one of its calls is fresh.
 */
import type { JSX } from "react";

import { wakeLine } from "../../lib/connector";
import type { DesktopSessionInfo } from "../../lib/ipc/connector";
import type { PoolRuleInfo } from "../../lib/ipc/pool-model";
import { roleLabel } from "../../lib/pools";

/** The desktop sessions drawn under a broad member's card: an App's own
 * (by its id); one an account, tool, Mac or Everyone brought in goes
 * under that kind's card when the pool has one of that kind, or the one
 * its account's label names; never under two. */
export function sessionsUnder(rule: PoolRuleInfo, sessions: readonly DesktopSessionInfo[], broad: readonly PoolRuleInfo[], labelOf: (rule: PoolRuleInfo) => string): DesktopSessionInfo[] {
  return sessions.filter((one) => {
    if (one.via === "app") return `app:${one.app_id}` === rule.member;
    if (one.via !== rule.kind) return false;
    const alike = broad.filter((other) => other.kind === rule.kind);
    return alike.length === 1 || (one.account !== null && labelOf(rule) === one.account);
  });
}

export function DesktopSessions({ sessions }: { sessions: readonly DesktopSessionInfo[] }): JSX.Element | null {
  if (sessions.length === 0) return null;
  return (
    <ul className="pointer-events-auto -mt-0.5 mb-1.5 ml-3 flex flex-col gap-1 border-l border-hairline pl-2" aria-label="Its sessions in this pool" data-testid="canvas-desktop-sessions">
      {sessions.map((one) => (
        <li key={one.member} className="flex flex-col rounded-lg bg-surface px-2.5 py-1.5 shadow-card" data-testid="canvas-desktop-session" data-member={one.member}>
          <span className="flex items-center gap-1.5">
            <span aria-hidden="true" className={"size-1.5 shrink-0 rounded-full " + (one.session_id ? "bg-success" : "bg-muted")} />
            <span className="min-w-0 flex-1 truncate text-meta font-medium text-ink" title={one.name}>
              {one.name}
            </span>
            <span className="shrink-0 text-label text-muted">{roleLabel(one.role)}</span>
          </span>
          <span className="text-label text-ink-2">{wakeLine(one.wake)}</span>
        </li>
      ))}
    </ul>
  );
}
