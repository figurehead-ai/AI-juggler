/**
 * 0.3 polish — a session header's one Account chip (the review: the
 * header said its profile twice and its place three times): its profile's
 * badge and name, ▾. It opens a menu that says which account and profile
 * the session uses and in which folder, with the door to the profile's
 * page (the chip it replaced opened nothing; the rail's profile header
 * and Accounts lead to the same page).
 */
import { useRef, useState, type JSX } from "react";

import { ProfileBadge } from "../../components/PlaceMarks";
import { MenuItem, MenuSeparator, RowMenu, type MenuAt } from "../../components/RowMenu";
import { ChoiceLine } from "../../components/controls";
import { IconChevronDown } from "../../components/icons-controls";
import { IconProfiles } from "../../components/icons";
import { useActionHost } from "../../components/ItemMenu";
import { profileColor } from "../../lib/profile-marks";
import { shortPath } from "../../screens/AccountManage";
import { accountLine, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import type { SessionModel } from "./model";

/** The profile's name as the rail's badge reads it (its place's, else
 * its folder's): one colour and monogram everywhere. */
export function profileName(model: SessionModel): string {
  const place = model.places[0];
  return place?.profile ?? place?.instance ?? model.profileLabel;
}

/** "Work Max · w…@example.com · max-20x": the account behind the
 * profile, once it's signed in. */
export function accountWordsOf(account: SubscriptionInfo | undefined): string | null {
  if (!account || account.state !== "signed_in") return null;
  const line = accountLine(account);
  return `${accountName(account)}${line ? ` · ${line}` : ""}`;
}

/** One fact in the menu: a small muted label over its value. */
function Fact({ label, value, mono = false, testId }: { label: string; value: string; mono?: boolean; testId: string }): JSX.Element {
  return (
    <span className="flex min-w-0 flex-col px-2 py-1" data-testid={testId}>
      <span className="text-label text-muted">{label}</span>
      <span className={"break-words text-meta text-ink " + (mono ? "font-mono" : "")}>{value}</span>
    </span>
  );
}

export function AccountMenu({ model, account }: { model: SessionModel; account: SubscriptionInfo | undefined }): JSX.Element {
  const host = useActionHost();
  const [at, setAt] = useState<MenuAt | null>(null);
  const button = useRef<HTMLButtonElement | null>(null);
  const badge = profileName(model);
  const words = accountWordsOf(account);
  const open = () => {
    const rect = button.current?.getBoundingClientRect();
    if (rect) setAt({ x: rect.left, y: rect.bottom + 6, above: rect.top - 6, right: rect.right });
  };
  const openPage = host.openAccountById && model.profileId ? () => host.openAccountById!(model.profileId) : null;
  return (
    <>
      <button
        ref={button}
        type="button"
        aria-haspopup="menu"
        aria-expanded={at !== null}
        aria-label={`Account: ${model.profileLabel}`}
        title={words ? `Account: ${words}` : `Profile: ${model.profileLabel}`}
        onClick={() => (at ? setAt(null) : open())}
        className="inline-flex min-w-0 max-w-full shrink items-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-0.5 text-meta font-medium leading-5 text-ink outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
        style={{ borderColor: profileColor(badge) }}
        data-testid="session-account-menu"
      >
        <ProfileBadge name={badge} size={16} />
        <span className="truncate">{model.profileLabel}</span>
        <IconChevronDown width={12} height={12} aria-hidden="true" className="shrink-0 text-muted" />
      </button>
      {at && (
        <RowMenu heading="Account" ariaLabel={`Account: ${model.profileLabel}`} at={at} onClose={() => setAt(null)} wide testId="session-account-panel">
          {/* The account: its name, then its sign-in and plan; on the
              menu's one text column (the profile's badge is the chip's). */}
          <span className="flex min-w-0 flex-col px-2 py-1">
            <span className="truncate text-meta font-semibold text-ink">{account && words ? accountName(account) : model.profileLabel}</span>
            {account && words && accountLine(account) && <span className="truncate text-label text-muted">{accountLine(account)}</span>}
          </span>
          {/* Its profile, when the account's name doesn't already say it. */}
          {(!account || !words || accountName(account) !== model.profileLabel) && <Fact label="Profile" value={model.profileLabel} testId="session-account-profile" />}
          {model.cwd && <Fact label="Folder" value={shortPath(model.cwd)} mono testId="session-account-folder" />}
          {openPage && (
            <>
              <MenuSeparator />
              <MenuItem onClick={() => (setAt(null), openPage())} testId="session-account-open">
                <ChoiceLine icon={IconProfiles} label={`Open ${model.profileLabel}'s page`} />
              </MenuItem>
            </>
          )}
        </RowMenu>
      )}
    </>
  );
}
