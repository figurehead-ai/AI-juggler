/**
 * 0.3 polish — a session header's one App chip (the review: the header
 * said its profile twice and its place three times): its profile's badge
 * and its App's name (D-62: "Claude Code · Work Max", "Claude app ·
 * b-work"), ▾. It opens a menu that says which App, which account (the
 * subscription behind it) and which working folder, with the door to the
 * App's page (the chip it replaced opened nothing; the rail's App header
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
import { sessionAppName, type SessionModel } from "./model";
import { IconGauge } from "../../components/usage/controls";
import { appIdOfRailKey, openApp } from "../../lib/apps";
import { openAccount } from "../../lib/accounts-page";
import { desktopAppKey } from "../../lib/app-name";
import { useKnownPlaces } from "../../lib/places";
import type { KnownPlaceInfo } from "../../lib/ipc/places";

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

/** The App a session runs in: the desktop app and profile it's open in,
 * else its folder's CLI App. */
export function appIdOfSession(model: SessionModel, places: readonly KnownPlaceInfo[] | null): string | null {
  const place = model.places.find((one) => one.kind === "desktop");
  if (place) return appIdOfRailKey(desktopAppKey(place.app_id ?? null, place.app, place.instance), null, places);
  return model.profileId ? `cli:${model.profileId}` : null;
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
  const app = sessionAppName(model);
  const open = () => {
    const rect = button.current?.getBoundingClientRect();
    if (rect) setAt({ x: rect.left, y: rect.bottom + 6, above: rect.top - 6, right: rect.right });
  };
  // D-62 (AJ-E3206, A3): the App it runs in (a Claude app profile, else
  // its folder's CLI App) and the Account behind it, a click each.
  const places = useKnownPlaces();
  const appId = appIdOfSession(model, places);
  const openPage = appId ? () => openApp(appId) : host.openAccountById && model.profileId ? () => host.openAccountById!(model.profileId) : null;
  const accountId = account?.state === "signed_in" ? (account.account_id ?? null) : null;
  return (
    <>
      <button
        ref={button}
        type="button"
        aria-haspopup="menu"
        aria-expanded={at !== null}
        aria-label={`App: ${app}`}
        title={words ? `${app}\nAccount: ${words}` : app}
        onClick={() => (at ? setAt(null) : open())}
        className="inline-flex min-w-0 max-w-full shrink items-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-0.5 text-meta font-medium leading-5 text-ink outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
        style={{ borderColor: profileColor(badge) }}
        data-testid="session-account-menu"
      >
        <ProfileBadge name={badge} size={16} />
        <span className="truncate">{app}</span>
        <IconChevronDown width={12} height={12} aria-hidden="true" className="shrink-0 text-muted" />
      </button>
      {at && (
        <RowMenu heading="App" ariaLabel={`App: ${app}`} at={at} onClose={() => setAt(null)} wide testId="session-account-panel">
          {/* The App: its name (its profile is inside it), on the menu's
              one text column (the profile's badge is the chip's). */}
          <span className="flex min-w-0 flex-col px-2 py-1">
            <span className="truncate text-meta font-semibold text-ink" data-testid="session-account-profile">{app}</span>
          </span>
          {/* Its account: the subscription behind it, once signed in. */}
          {account && words && <Fact label="Account" value={words} testId="session-account-account" />}
          {model.cwd && <Fact label="Working folder" value={shortPath(model.cwd)} mono testId="session-account-folder" />}
          {openPage && (
            <>
              <MenuSeparator />
              <MenuItem onClick={() => (setAt(null), openPage())} testId="session-account-open">
                <ChoiceLine icon={IconProfiles} label={`Open ${app}'s page`} />
              </MenuItem>
            </>
          )}
          {accountId && account && (
            <MenuItem onClick={() => (setAt(null), openAccount(accountId))} testId="session-account-open-account">
              <ChoiceLine icon={IconGauge} label={`Open ${accountName(account)}'s page`} />
            </MenuItem>
          )}
        </RowMenu>
      )}
    </>
  );
}
