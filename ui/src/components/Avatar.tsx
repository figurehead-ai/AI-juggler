/**
 * AJ-E1532 with AJ-E3206 — an Account's avatar: round, with the initials
 * that tell it from the user's others, and its name as a tooltip: the
 * Account's own (looked up when hovered, from the Accounts already read),
 * else the label it was given. Decorative: whatever it sits in names it
 * in words ("on Work Max").
 *
 * Colour belongs to Apps (the maintainer, 2026-11-03): wherever an App is
 * shown beside it (a session's row, the rail, History, a session's page)
 * the avatar is neutral, a hollow ring when small and grey with its
 * initials when large, so a row wears one colour, its App's. Only where
 * Accounts stand alone (the Runway, Usage, the Accounts screen, the
 * Account's page) does it wear the Account's own colour (`tone="colour"`).
 * A ring says the account serves a live session there, the same live
 * colour sessions use.
 */
import type { JSX, PointerEvent } from "react";
import { accountInitials } from "../lib/palette";
import { accountName, accountOfProfile, cachedSubscriptions, loadSubscriptions } from "../lib/subscriptions";
import { accountColor, accountInk, useMarks, type AccountMark } from "../lib/marks";

/** Below this size an avatar is its shape alone (a ring, or its colour
 * where it has one): its letters would be under 12 px, and its name is
 * always beside it or on hover. */
export const AVATAR_LETTERS_FROM = 24;

export function AccountAvatar({
  account,
  label,
  size = 18,
  live = false,
  tone = "neutral",
}: {
  /** The Account: a row of the Accounts list, its slot, or a folder (the
   * Account it was last read as). */
  account: AccountMark;
  label: string;
  size?: number;
  live?: boolean;
  /** Its own colour only where no App is shown beside it. */
  tone?: "neutral" | "colour";
}): JSX.Element {
  useMarks();
  const color = accountColor(account);
  const folder = typeof account === "object" && "folder" in account ? account.folder : null;
  const letters = size >= AVATAR_LETTERS_FROM;
  // A folder's avatar names the Account it is signed in to, not the folder.
  const nameOnHover = (event: PointerEvent<HTMLSpanElement>) => {
    if (folder === null) return;
    const rows = cachedSubscriptions();
    if (rows === null) return void loadSubscriptions();
    const owner = accountOfProfile(rows, folder);
    event.currentTarget.title = owner ? accountName(owner) : label;
  };
  return (
    <span
      aria-hidden="true"
      title={label}
      onPointerEnter={nameOnHover}
      data-testid="account-avatar"
      data-live={live ? "true" : undefined}
      data-tone={tone}
      data-color={tone === "colour" ? color : undefined}
      className={
        "inline-flex shrink-0 select-none items-center justify-center rounded-full font-semibold leading-none " +
        (tone === "colour" ? "" : letters ? "border border-hairline bg-surface-2 text-ink-2 " : "border-[1.5px] border-muted ") +
        (live ? "avatar-live" : "")
      }
      data-letters={letters ? "true" : undefined}
      style={{ width: size, height: size, fontSize: Math.max(12, Math.round(size * 0.46)), ...(tone === "colour" ? { background: color, color: accountInk(account) } : {}) }}
    >
      {letters ? accountInitials(folder, label) : null}
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
