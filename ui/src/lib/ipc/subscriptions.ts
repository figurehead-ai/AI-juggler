/**
 * AJ-E2109 (D-35) — accounts are subscriptions; profiles sit under them.
 * The backend groups the stored profiles by their sign-in when asked
 * (nothing is stored). The preview and the tests read the fixture's
 * `subscriptions` instead, or one account per signed-in profile.
 */
import { call } from "./transport";
import { accountsList, previewFolders } from "./accounts";
import { hasLiveBackend, signedIn } from "../bridge";
import type { SubscriptionInfo } from "../../types/generated/SubscriptionInfo";
import type { SubscriptionProfileInfo } from "../../types/generated/SubscriptionProfileInfo";
import type { TermChoiceInfo } from "../../types/generated/TermChoiceInfo";
import type { UnnamedSubscription } from "../../preview/names";
import { previewHooks } from "../preview-hooks";

export type { SubscriptionInfo, SubscriptionProfileInfo, TermChoiceInfo };

/** D-62 (AJ-E3202 req. 4): what an Account is set with, once. */
export type Term = "plan" | "renewal" | "fee";

/** The preview's accounts: the fixture's, else one per signed-in profile
 * and the rest under "Not signed in", per tool. */
async function previewSubscriptions(): Promise<UnnamedSubscription[]> {
  const fixture = (window as unknown as { __AIJ_FIXTURE__?: { subscriptions?: UnnamedSubscription[] } }).__AIJ_FIXTURE__;
  if (fixture?.subscriptions) return fixture.subscriptions;
  const profiles = await accountsList();
  const member = (id: string, label: string): SubscriptionProfileInfo => ({ id, label, folder: `~/.claude-${id}`, plan: null, renewal_date: null, fee_usd: null });
  const base = { organisation: null, reason: null, plan: null, renewal_date: null, fee_usd: null, differs: [], choices: [] };
  const signedInRows: UnnamedSubscription[] = profiles
    .filter((p) => signedIn(p.login_state))
    .map((p) => ({ ...base, key: `k-${p.id}`, tool: p.provider, state: "signed_in", email: `${p.label.toLowerCase().replace(/\s+/g, "")[0] ?? "y"}…@example.com`, profiles: [{ ...member(p.id, p.label), plan: p.plan, renewal_date: p.renewal_date }], plan: p.plan, renewal_date: p.renewal_date }));
  const out = new Map<string, UnnamedSubscription>();
  for (const p of profiles.filter((row) => !signedIn(row.login_state))) {
    const row = out.get(p.provider) ?? { ...base, key: `out-${p.provider}`, tool: p.provider, state: "signed_out", email: null, profiles: [] };
    row.profiles.push(member(p.id, p.label));
    out.set(p.provider, row);
  }
  return [...signedInRows, ...out.values()];
}

/** The names the preview gives its accounts (no backend to keep them). */
const previewNames = new Map<string, string>();
/** The terms set in the preview, by account (a choice made is gone). */
const previewTerms = new Map<string, Partial<Record<Term, string | null>>>();

const TERM_FIELD = { plan: "plan", renewal: "renewal_date", fee: "fee_usd" } as const;

/** An account with the terms set on it in the preview. */
function withPreviewTerms(row: UnnamedSubscription): UnnamedSubscription {
  const set = row.account_id ? previewTerms.get(row.account_id) : undefined;
  if (!set) return row;
  const next: UnnamedSubscription = { ...row };
  for (const [term, value] of Object.entries(set) as [Term, string | null][]) {
    if (term === "fee") next.fee_usd = value === null ? null : Number(value);
    else next[TERM_FIELD[term]] = value;
  }
  next.choices = (row.choices ?? []).filter((choice) => !(choice.term in set));
  next.differs = row.differs.filter((term) => !(term in set));
  return next;
}

/** Every profile, grouped into the account it's signed in to. In the
 * preview, each signed-in account is stored under its key (D-62), and
 * named as the backend would name it (its names and terms as set here). */
export async function subscriptionsList(): Promise<SubscriptionInfo[]> {
  if (!hasLiveBackend()) {
    const rows = (await previewSubscriptions()).map((row) => {
      const id = previewHooks().slots.previewAccountId(row);
      return withPreviewTerms({ ...row, choices: row.choices ?? [], account_id: id, name: (id && previewNames.get(id)) ?? row.name ?? null });
    });
    return previewHooks().slots.previewAccountSlots(previewHooks().names.previewNamed(rows), previewFolders());
  }
  return call("subscriptions_list");
}

/** D-62 (AJ-E3205): names an Account (`null` or blank: its default). */
export async function subscriptionsRename(id: string, name: string | null): Promise<void> {
  if (!hasLiveBackend()) {
    const named = name?.trim() ?? "";
    if (named) previewNames.set(id, named);
    else previewNames.delete(id);
    return;
  }
  await call("subscriptions_rename", { id, name });
}

/** D-62 (AJ-E3202 req. 4): sets Account `id`'s plan, renewal date
 * (`YYYY-MM-DD`) or monthly fee (dollars), once for all its Apps; `null`
 * clears it. Where its Apps disagreed, this is the choice. */
export async function subscriptionsSetTerm(id: string, term: Term, value: string | null): Promise<void> {
  if (!hasLiveBackend()) {
    previewTerms.set(id, { ...previewTerms.get(id), [term]: value?.trim() ? value.trim() : null });
    return;
  }
  await call("subscriptions_set_term", { id, term, value });
}
