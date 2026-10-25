/**
 * AJ-E1532 — an account's avatar: round, in the account's colour, with
 * the initials that tell it from the user's other accounts, and its name
 * as a tooltip. Decorative: whatever it sits in names the account in
 * words ("on Work Max"). One component wherever an account shows — the
 * session rows, the project rows' stacks, the Accounts cards and the
 * page headers (D-25). A ring says the account serves a live session
 * there, the same live colour sessions use.
 */
import type { JSX } from "react";
import { accountColor, accountInitials } from "../lib/palette";

export function AccountAvatar({
  accountId,
  label,
  size = 18,
  live = false,
}: {
  accountId: string | null;
  label: string;
  size?: number;
  live?: boolean;
}): JSX.Element {
  return (
    <span
      aria-hidden="true"
      title={label}
      data-testid="account-avatar"
      data-live={live ? "true" : undefined}
      className={"inline-flex shrink-0 select-none items-center justify-center rounded-full font-semibold leading-none text-white " + (live ? "avatar-live" : "")}
      style={{ width: size, height: size, background: accountColor(accountId), fontSize: Math.max(8, Math.round(size * 0.46)) }}
    >
      {accountInitials(accountId, label)}
    </span>
  );
}

/** One account in a stack. */
export type StackedAccount = { id: string | null; label: string; live: boolean };

/** Up to `max` accounts' avatars, just overlapping (their initials stay
 * whole), then "+N" for the rest. */
export function AccountStack({ accounts, max = 3, size = 16 }: { accounts: StackedAccount[]; max?: number; size?: number }): JSX.Element | null {
  if (accounts.length === 0) return null;
  const shown = accounts.slice(0, max);
  const more = accounts.length - shown.length;
  return (
    <span aria-hidden="true" className="flex shrink-0 items-center" data-testid="account-stack">
      {shown.map((account, index) => (
        <span key={account.id ?? account.label} className="avatar-stacked rounded-full" style={{ marginLeft: index === 0 ? 0 : -Math.round(size / 7), zIndex: shown.length - index }}>
          <AccountAvatar accountId={account.id} label={account.label} size={size} live={account.live} />
        </span>
      ))}
      {more > 0 && <span className="ml-1 text-label font-medium tabular-nums text-muted">+{more}</span>}
    </span>
  );
}
