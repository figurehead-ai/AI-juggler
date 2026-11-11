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

/** A broad member's open sessions now, as Review and its canvas card say
 * it: "1 open now · new ones join", "none open now · new ones join". */
export function openNowWords(count: number): string {
  return `${count > 0 ? `${count} open now` : "none open now"} · new ones join`;
}
