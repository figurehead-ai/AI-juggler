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
  const member = (id: string, label: string): SubscriptionProfileInfo => ({ id, label, folder: `~/.claude-${id}`, plan: null, renewal_date: null, fee_usd: null });
  const base = { organisation: null, reason: null, plan: null, renewal_date: null, fee_usd: null, differs: [] };
  const signedInRows: SubscriptionInfo[] = profiles
    .filter((p) => signedIn(p.login_state))
    .map((p) => ({ ...base, key: `k-${p.id}`, tool: p.provider, state: "signed_in", email: `${p.label.toLowerCase().replace(/\s+/g, "")[0] ?? "y"}…@example.com`, profiles: [{ ...member(p.id, p.label), plan: p.plan }], plan: p.plan }));
  const out = new Map<string, SubscriptionInfo>();
  for (const p of profiles.filter((row) => !signedIn(row.login_state))) {
    const row = out.get(p.provider) ?? { ...base, key: `out-${p.provider}`, tool: p.provider, state: "signed_out", email: null, profiles: [] };
    row.profiles.push(member(p.id, p.label));
    out.set(p.provider, row);
  }
  return [...signedInRows, ...out.values()];
}

/** The names the preview gives its accounts (no backend to keep them). */
const previewNames = new Map<string, string>();

/** Every profile, grouped into the account it's signed in to. In the
 * preview, each signed-in account is stored under its key (D-62). */
export async function subscriptionsList(): Promise<SubscriptionInfo[]> {
  if (!hasLiveBackend()) {
    return (await previewSubscriptions()).map((row) => {
      const id = row.account_id ?? (row.state === "signed_in" ? `acct:preview:${row.key}` : null);
      return { ...row, account_id: id, name: (id && previewNames.get(id)) ?? row.name ?? null };
    });
  }
  return invoke<SubscriptionInfo[]>("subscriptions_list");
}

/** D-62 (AJ-E3205): names an Account (`null` or blank: its default). */
export async function subscriptionsRename(id: string, name: string | null): Promise<void> {
  if (!hasLiveBackend()) {
    const named = name?.trim() ?? "";
    if (named) previewNames.set(id, named);
    else previewNames.delete(id);
    return;
  }
  await invoke<void>("subscriptions_rename", { id, name });
}
