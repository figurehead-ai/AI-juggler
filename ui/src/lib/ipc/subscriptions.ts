/**
 * AJ-E2109 (D-35) — accounts are subscriptions; profiles sit under them.
 * The backend groups the stored profiles by their sign-in when asked
 * (nothing is stored). The preview and the tests read the fixture's
 * `subscriptions` instead, or one account per signed-in profile.
 */
import { invoke } from "./transport";
import { accountsList, hasLiveBackend, signedIn } from "../bridge";
import type { SubscriptionInfo } from "../../types/generated/SubscriptionInfo";
import type { SubscriptionProfileInfo } from "../../types/generated/SubscriptionProfileInfo";

export type { SubscriptionInfo, SubscriptionProfileInfo };

/** The preview's accounts: the fixture's, else one per signed-in profile
 * and the rest under "Not signed in", per tool. */
async function previewSubscriptions(): Promise<SubscriptionInfo[]> {
  const fixture = (window as unknown as { __AIJ_FIXTURE__?: { subscriptions?: SubscriptionInfo[] } }).__AIJ_FIXTURE__;
  if (fixture?.subscriptions) return fixture.subscriptions;
  const profiles = await accountsList();
  const member = (id: string): SubscriptionProfileInfo => ({ id, folder: `~/.claude-${id}`, plan: null, renewal_date: null, fee_usd: null });
  const base = { organisation: null, reason: null, plan: null, renewal_date: null, fee_usd: null, differs: [] };
  const signedInRows: SubscriptionInfo[] = profiles
    .filter((p) => signedIn(p.login_state))
    .map((p) => ({ ...base, key: `k-${p.id}`, tool: p.provider, state: "signed_in", email: `${p.label.toLowerCase().replace(/\s+/g, "")[0] ?? "y"}…@example.com`, profiles: [{ ...member(p.id), plan: p.plan }], plan: p.plan }));
  const out = new Map<string, SubscriptionInfo>();
  for (const p of profiles.filter((row) => !signedIn(row.login_state))) {
    const row = out.get(p.provider) ?? { ...base, key: `out-${p.provider}`, tool: p.provider, state: "signed_out", email: null, profiles: [] };
    row.profiles.push(member(p.id));
    out.set(p.provider, row);
  }
  return [...signedInRows, ...out.values()];
}

/** Every profile, grouped into the account it's signed in to. */
export async function subscriptionsList(): Promise<SubscriptionInfo[]> {
  if (!hasLiveBackend()) return previewSubscriptions();
  return invoke<SubscriptionInfo[]>("subscriptions_list");
}
