/**
 * The rc27 P1 walk: picking an App in the New pool wizard promised that
 * "its sessions here join by themselves, now and later", and the pool's
 * card then read "7 sessions now" for one open session and six long
 * ended. "Now" is the open ones, said alike in the wizard's Review and on
 * the canvas's card ("1 open now · new ones join", the backend's
 * `PoolRuleInfo.sessions`); an App member still covers its ended
 * sessions, which the List view keeps.
 *
 * What a broad member brings in now, by the resolver's rule
 * (`juggler_core::pools`): an open session in the pool's scope that no
 * more specific member names (a session › App › Account › Tool › Mac ›
 * everyone), as the wizard's candidates match them (`broadCandidates`).
 */
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { AppModel } from "./apps";
import type { Draft } from "./new-pool";
import { sessionState } from "./rail-items";
import type { SubscriptionInfo } from "./subscriptions";
import { listWords } from "./account-terms";

/** How specific each kind of member is: the lower wins. */
const RANK: Record<string, number> = { session: 0, app: 1, account: 2, tool: 3, mac: 4, all: 5 };

export interface OpenNowInput {
  apps: readonly Pick<AppModel, "id" | "folder">[];
  subscriptions: readonly Pick<SubscriptionInfo, "account_id" | "profiles">[];
}

/** The kind of a member ref (`app:cli:work` → `app`, `all` → `all`). */
const kindOf = (ref: string) => (ref === "all" ? "all" : ref.slice(0, ref.indexOf(":")));

/** Whether member `ref` matches `session`. */
function matches(ref: string, session: InstanceInfo, input: OpenNowInput): boolean {
  const kind = kindOf(ref);
  const id = ref.slice(kind.length + 1);
  switch (kind) {
    case "all":
      return true;
    case "session":
      return session.id === id;
    case "app": {
      const folder = input.apps.find((app) => app.id === id)?.folder;
      return !!folder && session.account_id === folder.id;
    }
    case "account":
      return !!session.account_id && !!input.subscriptions.find((account) => account.account_id === id)?.profiles.some((profile) => profile.id === session.account_id);
    case "tool":
      return session.provider === id;
    case "mac":
      return session.machine_id === id;
    default:
      return false;
  }
}

/** Each draft member's open sessions here now, by its ref. */
export function openNowByMember(draft: Pick<Draft, "every" | "projects" | "members">, sessions: readonly InstanceInfo[], input: OpenNowInput): Map<string, number> {
  const counts = new Map<string, number>(draft.members.map((member) => [member.ref, 0]));
  const members = [...draft.members].sort((a, b) => (RANK[kindOf(a.ref)] ?? 9) - (RANK[kindOf(b.ref)] ?? 9));
  for (const session of sessions) {
    if (sessionState(session).state === "ended") continue;
    if (!draft.every && !draft.projects.includes(session.project_id ?? null)) continue;
    const by = members.find((member) => matches(member.ref, session, input));
    if (by) counts.set(by.ref, (counts.get(by.ref) ?? 0) + 1);
  }
  return counts;
}

/** Everyone in scope's card (round 3, #6): "Every new session in web joins
 * here, now and later", then its open ones. */
export function everyoneJoins(count: number, where?: string): string {
  const place = !where ? "these projects" : where === "every project" ? "any project" : where;
  return `Every new session in ${place} joins here, now and later${count > 0 ? ` · ${count} open now` : ""}`;
}

/** A broad member's open sessions now, as Review and its diagram card say
 * it: "1 open now · new ones in web join", "No session open yet · new ones
 * in web join" (round 2, #15: "none open now" read as broken). */
export function openNowWords(count: number, where?: string, quiet = 0): string {
  const joins = !where ? "new ones join" : where === "every project" ? "new ones in any project join" : `new ones in ${where} join`;
  // AJ-E3013: a desktop App's sessions in the pool that haven't called
  // lately are in it, drawn under its card: never "No session open yet".
  const none = quiet > 0 ? `${quiet} in the pool, quiet lately` : "No session open yet";
  return `${count > 0 ? `${count} open now` : none} · ${joins}`;
}

/** AJ-E3014 item 4: who joins now and who later, in a line, for New
 * pool's side panel (P1's sheet: only Review said it). Now: the sessions
 * open here now that a member brings in (`openNow`); later: the members
 * whose new sessions join by themselves, or no one. "Now: 2 open
 * sessions · Later: every new session in web". */
export function joinsLine(members: readonly { ref: string; label: string; later: boolean }[], openNow: ReadonlyMap<string, number>, where: string): string {
  const open = members.reduce((sum, member) => sum + (openNow.get(member.ref) ?? 0), 0);
  const now = open === 0 ? "no session open" : open === 1 ? "1 open session" : `${open} open sessions`;
  const laters = members.filter((member) => member.ref !== "all" && member.later).map((member) => member.label);
  const later = members.some((member) => member.ref === "all")
    ? `every new session in ${where}`
    : laters.length > 0
      ? `new sessions of ${listWords(laters)} in ${where}`
      : "no one by itself: add sessions on its diagram";
  return `Now: ${now} · Later: ${later}`;
}
