/**
 * Accounts: the list, add, rename, remove, default, sign-in, and the
 * per-account settings (fee, renewal, cleanup).
 */
import { call, liveBackend as hasBackend } from "./transport";
import { refused, signedIn } from "../bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { AccountRemovedInfo } from "../../types/generated/AccountRemovedInfo";
import type { SubscriptionInfo } from "../../types/generated/SubscriptionInfo";
import { previewHooks } from "../preview-hooks";

/** ADR-0018: a copied account switches to its original folder, used
 * where it is; the copy stays on disk. Mock: the account, in place. */
export async function accountUseOriginal(accountId: string): Promise<AccountInfo> {
  if (!hasBackend) {
    const fail = (window as unknown as { __AIJ_USE_ORIGINAL_FAIL__?: string }).__AIJ_USE_ORIGINAL_FAIL__;
    if (fail) throw { code: "validation", retryable: false, user_message: fail };
    const accounts = await accountsList();
    const account = accounts.find((row) => row.id === accountId);
    if (!account) throw { code: "not_found", retryable: false, user_message: "That account isn't there any more." };
    account.in_place = true; // the preview's list reflects it on refresh
    return account;
  }
  return call("account_use_original", { accountId });
}

/** AJ-E1501: rename an account and set its plan (blank clears it). */
export async function accountRename(accountId: string, label: string, plan: string | null): Promise<AccountInfo> {
  if (!hasBackend) {
    const accounts = await accountsList();
    const account = accounts.find((row) => row.id === accountId);
    if (!account) throw { code: "not_found", retryable: false, user_message: "That account isn't there any more." };
    const name = label.trim();
    if (name === "") throw refused("Give the account a name");
    const other = accounts.find((row) => row.id !== accountId && row.label.trim().toLowerCase() === name.toLowerCase());
    if (other) throw refused(`Another account is already called «${other.label}» — pick another name`);
    account.label = name;
    account.plan = plan && plan.trim() !== "" ? plan.trim() : null;
    for (const recipe of previewHooks().fixtures.MOCK_PROFILES) if (recipe.account_id === accountId) recipe.account_label = name;
    for (const row of previewHooks().fixtures.MOCK_SESSIONS) if (row.account_id === accountId) row.account_label = name;
    return account;
  }
  return call("account_rename", { accountId, label, plan });
}

/** AJ-E1501: remove (forget) an account. `folder` decides a copy's
 * folder: `keep` it, or move it to the Trash. An account used where it
 * is always keeps its folder. */
export async function accountRemove(accountId: string, folder: "keep" | "trash"): Promise<AccountRemovedInfo> {
  if (!hasBackend) {
    const fail = (window as unknown as { __AIJ_REMOVE_FAIL__?: string }).__AIJ_REMOVE_FAIL__;
    if (fail) throw refused(fail);
    const accounts = previewHooks().fixtures.seeded.accounts ?? previewHooks().fixtures.MOCK_ACCOUNTS;
    const index = accounts.findIndex((row) => row.id === accountId);
    const account = accounts[index];
    if (!account) throw { code: "not_found", retryable: false, user_message: "That account isn't there any more." };
    const open = previewHooks().fixtures.MOCK_SESSIONS.filter(
      (row) => row.account_id === accountId && ["starting", "running", "waiting_user", "stuck", "quota_blocked"].includes(row.status),
    ).map((row) => row.git_branch ?? ((row.cwd ?? "").split("/").pop() || row.account_label));
    if (open.length > 0) {
      const names = [...new Set(open)].sort().join(", ");
      throw refused(
        `«${account.label}» has ${open.length === 1 ? "a session" : `${open.length} sessions`} open: ${names}. Stop ${open.length === 1 ? "it" : "them"} first, then remove the account`,
      );
    }
    accounts.splice(index, 1);
    return { id: account.id, label: account.label, folder: account.in_place ? "untouched" : folder === "trash" ? "trashed" : "kept" };
  }
  return call("account_remove", { accountId, folder });
}

/** AJ-E1501: make an account the default for new sessions (`null`
 * clears it: the most recently used account leads again). */
export async function accountSetDefault(accountId: string | null): Promise<AccountInfo[]> {
  if (!hasBackend) {
    const accounts = await accountsList();
    for (const row of accounts) row.is_default = row.id === accountId;
    return accounts;
  }
  return call("account_set_default", { accountId });
}

/** AJ-E1501: the account a project's new sessions start with (`null`
 * clears it). */
export async function projectSetDefaultAccount(projectId: string, accountId: string | null): Promise<void> {
  if (!hasBackend) {
    const project = previewHooks().fixtures.MOCK_PROJECTS.find((row) => row.id === projectId);
    if (project) project.default_account_id = accountId;
    return;
  }
  return call("project_set_default_account", { projectId, accountId });
}

export async function accountsList(): Promise<AccountInfo[]> {
  if (!hasBackend) {
    const hang = previewHooks().support.loadHang();
    if (hang) return hang;
    // A new array each time (the rows are shared): a change made in
    // preview mode re-renders like a real refresh does.
    return [...previewFolders()];
  }
  return call("accounts_list");
}

/** The preview's folders (the rows are shared), each with the colours
 * the backend would give its CLI App and its Account (AJ-E3206: filled
 * once, kept). */
export function previewFolders(): AccountInfo[] {
  const subscriptions = (previewHooks().fixtures.seeded as { subscriptions?: SubscriptionInfo[] }).subscriptions;
  return previewHooks().slots.previewFolderSlots(previewHooks().fixtures.seeded.accounts ?? previewHooks().fixtures.MOCK_ACCOUNTS, subscriptions, signedIn);
}

export async function createAccount(
  harness: string,
  label: string,
  plan: string | null,
): Promise<AccountInfo> {
  if (!hasBackend) {
    return {
      id: `claude-${label.toLowerCase().replace(/\s+/g, "-")}`,
      provider: harness,
      label,
      plan,
      login_state: "logged_out",
      cleanup_opt_in: false,
      renewal_date: null,
      preview_opt_in: false,
      in_place: false,
    };
  }
  return call("account_create", { harness, label, plan });
}

/** Opens the vendor's own web sign-in page in the system browser. */
export async function loginOpenWeb(harness: string): Promise<void> {
  return call("login_open_web", { harness });
}

export async function accountLoginCommand(accountId: string): Promise<string> {
  if (!hasBackend) {
    return `CLAUDE_CONFIG_DIR='~/Library/Application Support/AI Juggler/accounts/claude-code/${accountId}' claude`;
  }
  return call("account_login_command", { accountId });
}

export async function accountLoginState(accountId: string): Promise<string> {
  if (!hasBackend) {
    // AJ-E308 test seam: specs inject the probed state.
    return (
      (window as unknown as { __AIJ_LOGIN_STATE__?: string }).__AIJ_LOGIN_STATE__ ??
      "logged_out"
    );
  }
  return call("account_login_state", { accountId });
}

/** AJ-E1001: forgets an Account's remembered identity (learned again at
 * its next launch, shown as "first seen"). */
export async function accountForgetIdentity(accountId: string): Promise<void> {
  if (!hasBackend) return;
  return call("account_forget_identity", { accountId });
}

/** AJ-E610: set/clear the user-entered flat fee (dual view ratio). */
export async function accountSetFee(account: string, feeUsd: number | null): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_SET_FEE__?: (a: string, f: number | null) => void };
    w.__AIJ_SET_FEE__?.(account, feeUsd);
    return;
  }
  await call("account_set_fee", { account, feeUsd });
}


/** AJ-E411 (D-12): set (validate server-side) or clear the user-entered renewal date. */
export async function accountSetRenewal(
  accountId: string,
  date: string | null,
): Promise<AccountInfo> {
  if (!hasBackend) {
    // AJ-E309: the wizard sets a date on accounts created in this
    // preview session — record it on a seam instead of throwing.
    const seams = window as unknown as { __AIJ_RENEWAL_SET__?: [string, string | null][] };
    seams.__AIJ_RENEWAL_SET__ = [...(seams.__AIJ_RENEWAL_SET__ ?? []), [accountId, date]];
    const account = previewHooks().fixtures.MOCK_ACCOUNTS.find((row) => row.id === accountId);
    if (!account) {
      return {
        id: accountId,
        provider: previewHooks().fixtures.PREVIEW_TOOL.id,
        label: accountId,
        plan: null,
        login_state: "logged_out",
        cleanup_opt_in: false,
        renewal_date: date,
        preview_opt_in: false, in_place: false,
      };
    }
    return { ...account, renewal_date: date };
  }
  return call("account_set_renewal", { accountId, date });
}

/** AJ-E410 (D-08): set the per-Account Tier-1 cleanup opt-in. */
export async function accountSetCleanup(accountId: string, optIn: boolean): Promise<AccountInfo> {
  if (!hasBackend) {
    const account = previewHooks().fixtures.MOCK_ACCOUNTS.find((row) => row.id === accountId);
    if (!account) throw new Error(`unknown account ${accountId}`);
    return { ...account, cleanup_opt_in: optIn };
  }
  return call("account_set_cleanup", { accountId, optIn });
}
