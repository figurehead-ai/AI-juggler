/**
 * 0.5.1 F27 (AJ-E1553, D-84's one place per thing): an App whose sign-in
 * runs or ended early is in the rail once, as its Sign-ins entry (it says
 * how the sign-in went and opens its page, with Try again), never again
 * as a Needs you line "… needs its sign-in" beside it. The bell and Home
 * keep the need; once the entry leaves (signed in, or dismissed), the
 * rail's Needs you line comes back while the App still needs it.
 * F34 (AJ-E3104): the entry is that need's one place in the rail, and the
 * rail's Needs you count still includes it (`held`), so the rail, the bell
 * and Home say one number.
 */
import { useMemo } from "react";
import { counted, noticeLevel } from "./attention";
import { useBackendNeeds, type BackendNeeds } from "./needs-you";
import { useSignIns } from "./sign-ins-store";
import type { NoticeInfo } from "./ipc/notices";

/** Whether a sign-in shows in the rail's Sign-ins: one that finished
 * leaves by itself. */
export function signInShown(signIn: { state?: string }): boolean {
  return signIn.state !== "signed_in";
}

/** The Apps (account ids) the rail's Sign-ins show. */
export function signInApps(signIns: readonly { account_id?: string; state?: string }[]): ReadonlySet<string> {
  return new Set(signIns.filter(signInShown).flatMap((one) => (one.account_id ? [one.account_id] : [])));
}

/** The rail's Needs you lines without a sign-in need its Sign-ins already show. */
export function withoutSignInsShown<T extends Pick<NoticeInfo, "kind" | "target">>(rows: T[], apps: ReadonlySet<string>): T[] {
  if (apps.size === 0) return rows;
  return rows.filter((row) => !(row.kind === "signin" && row.target.account_id && apps.has(row.target.account_id)));
}

/** The Apps the rail's Sign-ins show now. */
function useSignInApps(): ReadonlySet<string> {
  const signIns = useSignIns();
  return useMemo(() => signInApps(signIns), [signIns]);
}

/** The rail's Needs you: its rows, and the counted needs (L2, L3) its
 * Sign-ins entries show in their place (listed there, counted here: F34). */
export interface RailNeeds extends BackendNeeds {
  held: NoticeInfo[];
}

const NONE: NoticeInfo[] = [];

/** The rail's Needs you (both boards): the backend's, without a sign-in
 * need its Sign-ins already show; those are `held`, still counted. */
export function useRailNeeds(): RailNeeds {
  const needs = useBackendNeeds();
  const apps = useSignInApps();
  return useMemo(() => {
    const rows = withoutSignInsShown(needs.rows, apps);
    if (rows.length === needs.rows.length) return { ...needs, held: NONE };
    const kept = new Set(rows);
    return { ...needs, rows, held: needs.rows.filter((row) => !kept.has(row) && counted(noticeLevel(row))) };
  }, [needs, apps]);
}
