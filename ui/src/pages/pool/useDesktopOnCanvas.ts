/**
 * Round 7 #7: a pool's desktop sessions on its diagram — those an App
 * brings in, and those named on their own (Watched or connected) — by
 * their member refs: the canvas is read for them too (their lines), and
 * each is named as its row says it (a line's words, the inspector).
 */
import { useMemo } from "react";

import { useDesktopSessions } from "../../lib/connector";
import type { DesktopSessionInfo } from "../../lib/ipc/connector";
import type { WatchedMemberInfo } from "../../lib/ipc/pool-model";

export function useDesktopOnCanvas(poolId: string, watched: readonly WatchedMemberInfo[]): { sessions: readonly DesktopSessionInfo[]; refs: string[]; names: ReadonlyMap<string, string> } {
  const sessions = useDesktopSessions(poolId);
  const own = useMemo(() => watched.filter((one) => one.member.startsWith("conversation:")), [watched]);
  const refs = useMemo(() => [...new Set([...sessions.map((one) => one.member), ...own.map((one) => one.member)])], [sessions, own]);
  const names = useMemo(() => new Map<string, string>([...own.map((one) => [one.member, one.title ?? "A session in another app"] as const), ...sessions.map((one) => [one.member, one.name] as const)]), [sessions, own]);
  return { sessions, refs, names };
}
