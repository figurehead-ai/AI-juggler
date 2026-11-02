/**
 * AJ-E1532 with AJ-E3206 — an Account's avatar: round, in the Account's
 * one colour (lib/marks: stored once per Account, the same on its cards,
 * its page, Usage and every session on it), with the initials that tell
 * it from the user's others, and its name as a tooltip. Decorative:
 * whatever it sits in names it in words ("on Work Max"). A session's row
 * shows the Account behind it by its folder (the Account it was last read
 * as). An App is never drawn so: its mark is the dot beside its icon
 * (`ProfileBadge`). A ring says the account serves a live session there,
 * the same live colour sessions use.
 */
import type { JSX } from "react";
import { accountInitials } from "../lib/palette";
import { accountColor, useMarks, type AccountMark } from "../lib/marks";

/** Below this size an avatar is its colour alone: its letters would be
 * under 12 px, and its name is always beside it. */
export const AVATAR_LETTERS_FROM = 24;

export function AccountAvatar({
  account,
  label,
  size = 18,
  live = false,
}: {
  /** The Account: a row of the Accounts list, its slot, or a folder (the
   * Account it was last read as). */
  account: AccountMark;
  label: string;
  size?: number;
  live?: boolean;
}): JSX.Element {
  useMarks();
  const color = accountColor(account);
  const folder = typeof account === "object" && "folder" in account ? account.folder : null;
  return (
    <span
      aria-hidden="true"
      title={label}
      data-testid="account-avatar"
      data-live={live ? "true" : undefined}
      data-color={color}
      className={"inline-flex shrink-0 select-none items-center justify-center rounded-full font-semibold leading-none text-white " + (live ? "avatar-live" : "")}
      data-letters={size >= AVATAR_LETTERS_FROM ? "true" : undefined}
      style={{ width: size, height: size, background: color, fontSize: Math.max(12, Math.round(size * 0.46)) }}
    >
      {size >= AVATAR_LETTERS_FROM ? accountInitials(folder, label) : null}
    </span>
  );
}

/** One account in a stack: by its folder (`id`). */
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
          <AccountAvatar account={{ folder: account.id }} label={account.label} size={size} live={account.live} />
        </span>
      ))}
      {more > 0 && <span className="ml-1 text-label font-medium tabular-nums text-muted">+{more}</span>}
    </span>
  );
}
