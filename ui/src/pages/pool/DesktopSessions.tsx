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

/** How many of them haven't called lately (one that has is an attested
 * session, already in its member's open count). */
export const quiet = (sessions: readonly DesktopSessionInfo[]) => sessions.filter((one) => one.session_id === null).length;

export function DesktopSessions({ sessions }: { sessions: readonly DesktopSessionInfo[] }): JSX.Element | null {
  if (sessions.length === 0) return null;
  return (
    <ul className="pointer-events-auto -mt-0.5 mb-1.5 ml-3 flex flex-col gap-1 border-l border-hairline pl-2" aria-label="Its sessions in this pool" data-testid="canvas-desktop-sessions">
      {sessions.map((one) => {
        const state = one.session_id ? "Active lately" : "Quiet lately";
        return (
          <li key={one.member} className="flex flex-col gap-0.5 rounded-lg bg-surface px-2.5 py-1.5 shadow-card" data-testid="canvas-desktop-session" data-member={one.member} data-desktop-ref={one.member}>
            <span className="flex items-start gap-1.5">
              <span aria-hidden="true" title={state} className={"mt-1.5 size-1.5 shrink-0 rounded-full " + (one.session_id ? "bg-success" : "bg-muted")} />
              <span className="sr-only">{state}: </span>
              <span className="line-clamp-2 min-w-0 text-meta font-medium text-ink [overflow-wrap:normal]" title={one.name}>
                {one.name}
              </span>
            </span>
            <span className="text-label text-ink-2">
              {roleLabel(one.role)} · {wakeLine(one.wake).toLowerCase()}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
