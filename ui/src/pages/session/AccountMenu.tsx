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
import { useMarks } from "../../lib/marks";
import { shortPath } from "../../screens/AccountManage";
import { accountLine, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import { sessionAppWords, type SessionModel } from "./model";
import { IconGauge } from "../../components/usage/controls";
import { appIdOfRailKey, openApp } from "../../lib/apps";
import { openAccount } from "../../lib/accounts-page";
import { appName, appShortName, cliAppId, desktopAppKey, desktopProfile, ownDesktopAppKey, type AppWords } from "../../lib/app-name";
import { appByConversation, appByFolder, appOfConversation, knownAppWord, knownKey } from "../../lib/rail-apps";
import { toolSpec } from "../../lib/tools";
import { useKnownPlaces } from "../../lib/places";
import type { KnownPlaceInfo } from "../../lib/ipc/places";

/** The profile's name as the rail's badge reads it (its place's, else
 * its folder's). */
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

/** The desktop profile a session is that profile's by, when no window
 * says so now (rc22, walk A3: the session's chip named and opened the CLI
 * App on the profile's folder, a second App that counted it too). The
 * rail's rule (`appKeyOf`), and the backend's (`places::app_of`):
 * - AI Juggler's, in the desktop app: the profile it was launched in
 *   (stored at launch), else the one profile on its folder (one AI
 *   Juggler runs in a terminal is its folder's CLI App);
 * - another app's, closed: the profile whose own records hold it (that
 *   copy only: a switch copies a conversation into another folder under
 *   the same id), else the one its tool says started it (one with neither
 *   ran in a terminal: its folder's CLI App);
 * - open: the desktop app its process runs in (none here), else its
 *   folder's CLI App (ADR-0032: never the one profile on its folder). */
type SessionFacts = Pick<SessionModel, "tool" | "profileId" | "conversationId" | "sessionId" | "places" | "desktop" | "startedIn" | "appId" | "owner">;
export function recordPlaceOf(model: SessionFacts, places: readonly KnownPlaceInfo[] | null): KnownPlaceInfo | undefined {
  if (!places || !model.profileId) return undefined;
  const byFolder = appByFolder(places);
  const sole = byFolder.get(`|${model.profileId}`);
  const find = (key: string | undefined) => (key ? places.find((place) => knownKey(place) === key) : undefined);
  if (model.sessionId !== null) {
    if (!model.desktop) return undefined;
    return find((model.appId ? byFolder.get(`@${model.appId}`) : undefined) ?? sole);
  }
  if (model.conversationId === null) return undefined;
  // ADR-0032: the backend's answer, when it gave one.
  if (model.owner) return model.owner.startsWith("app:") ? places.find((place) => place.id === model.owner) : undefined;
  const open = model.places[0];
  // Open: its process says (a desktop app's place, else a CLI App);
  // never the profile on its folder (ADR-0032).
  if (open) return undefined;
  const recorded = appOfConversation(appByConversation(places), model.profileId, `${model.tool}/${model.conversationId}`);
  return find(recorded ?? (model.startedIn ? sole : undefined));
}

/** The App a session runs in, as its mark keys it (AJ-E3206): the
 * desktop app and profile it's open in (the rail's key for it), else the
 * profile it's that profile's by ([`recordPlaceOf`]), else its folder's
 * CLI App; with neither, the session itself (a fallback by id). */
export function sessionAppMark(model: SessionFacts & Pick<SessionModel, "profileLabel">, places: readonly KnownPlaceInfo[] | null = null): string {
  const recorded = recordPlaceOf(model, places);
  // A desktop session AI Juggler started: its profile's key; with the
  // places read and none its own, its folder's CLI App (the backend's
  // order); before they're read, the rail's key for its app and folder.
  if (model.desktop) {
    if (recorded) return knownKey(recorded);
    if (places && model.profileId) return cliAppId(model.profileId);
    return ownDesktopAppKey(toolSpec(model.tool).host_app ?? null, model.tool, model.profileId, model.profileLabel);
  }
  // ADR-0032: a conversation the backend gave a CLI App is that one's,
  // whatever window it shows in.
  if (model.owner?.startsWith("cli:")) return model.owner;
  const place = model.places.find((one) => one.kind === "desktop");
  if (place) return desktopAppKey(place.app_id ?? null, place.app, place.instance);
  // Closed, in a desktop profile's own records or folder: that profile
  // (the go sitting: its chip read "Claude Code · aij-test").
  if (recorded) return knownKey(recorded);
  return model.profileId ? cliAppId(model.profileId) : `session:${model.sessionId ?? model.conversationId ?? ""}`;
}

/** The App a session runs in: the desktop app and profile it's open in,
 * else its folder's CLI App. */
export function appIdOfSession(model: Parameters<typeof sessionAppMark>[0], places: readonly KnownPlaceInfo[] | null): string | null {
  const mark = sessionAppMark(model, places);
  if (mark.startsWith("app|")) return appIdOfRailKey(mark, null, places);
  return mark.startsWith("cli:") ? mark : null;
}

/** A session's App, one read for every place that names it: its page's
 * id (`null`: none to open), its full name, that name's two parts and
 * its mark's key. */
function useSessionApp(model: SessionModel): { id: string | null; name: string; words: AppWords; mark: string } {
  const places = useKnownPlaces();
  const recorded = recordPlaceOf(model, places);
  const words = recorded ? { app: knownAppWord(recorded), profile: desktopProfile(recorded.instance, recorded.profile) } : sessionAppWords(model);
  return {
    id: appIdOfSession(model, places),
    name: appName(words.app, words.profile),
    words,
    mark: sessionAppMark(model, places),
  };
}

/** 0.4.1 (walk A3): an App's name in a narrow chip. The tool word gives
 * way before it, whole (D-100: it was cut to "C…"; the mark beside says
 * the tool), at the header row's second step (`useGiveWay`); its own name
 * last: the part that tells Apps apart, the rail's short name. */
function ChipName({ words }: { words: AppWords }): JSX.Element {
  const short = appShortName(words);
  if (words.profile === null) return <span className="truncate">{short}</span>;
  return (
    <span className="flex min-w-0 items-baseline">
      <span className="shrink-0 whitespace-pre data-gone:hidden" data-give-at="2" data-testid="session-app-word-part">
        <span data-testid="session-app-word">{words.app}</span>
        {" · "}
      </span>
      <span className="min-w-0 truncate" data-testid="session-app-short">
        {short}
      </span>
    </span>
  );
}

/** rc22 (walk A3): a session's App as a line's link, one click to its
 * page; plain words when it has none. */
export function SessionAppLink({ model }: { model: SessionModel }): JSX.Element {
  const app = useSessionApp(model);
  if (app.id === null) return <span data-testid="session-app-link">{app.name}</span>;
  const id = app.id;
  return (
    <button
      type="button"
      onClick={() => openApp(id)}
      title={`Open ${app.name}'s page`}
      className="max-w-full truncate rounded text-left text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="session-app-link"
    >
      {app.name}
    </button>
  );
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
  const chip = useRef<HTMLSpanElement | null>(null);
  const badge = profileName(model);
  useMarks();
  const { id: appId, name: app, words: appWords, mark } = useSessionApp(model);
  const words = accountWordsOf(account);
  const open = () => {
    // Under the whole chip, its left edge.
    const rect = (chip.current ?? button.current)?.getBoundingClientRect();
    if (rect) setAt({ x: rect.left, y: rect.bottom + 6, above: rect.top - 6, right: rect.right });
  };
  // D-62 (AJ-E3206, A3): the App it runs in (a Claude app profile, else
  // its folder's CLI App) and the Account behind it, a click each.
  const openPage = appId ? () => openApp(appId) : host.openAccountById && model.profileId ? () => host.openAccountById!(model.profileId) : null;
  const accountId = account?.state === "signed_in" ? (account.account_id ?? null) : null;
  return (
    <>
      {/* AJ-E3206 (walk A3): the App's name opens its page in one click;
          ▾ beside it opens the menu that says which App, Account and
          folder. A menu button, not a pill: beside the state's pill it
          must never read as a second state. The App's colour is its
          dot's alone. */}
      <span
        ref={chip}
        className="inline-flex min-w-0 max-w-full shrink items-center whitespace-nowrap rounded-md border border-hairline text-meta font-medium leading-5 text-ink"
        data-testid="session-app-chip"
      >
        <button
          type="button"
          aria-label={openPage ? `Open ${app}'s page` : `App: ${app}`}
          title={words ? `${app}\nAccount: ${words}` : app}
          onClick={() => (openPage ? openPage() : at ? setAt(null) : open())}
          className="inline-flex min-w-0 shrink items-center gap-1.5 rounded-l-md py-0.5 pl-2 pr-1 outline-none transition-colors hover:bg-surface-2 hover:underline focus-visible:ring-2 focus-visible:ring-accent/60"
          data-testid="session-app-page"
        >
          <ProfileBadge app={mark} name={badge} size={16} />
          <ChipName words={appWords} />
        </button>
        <button
          ref={button}
          type="button"
          aria-haspopup="menu"
          aria-expanded={at !== null}
          aria-label={`App: ${app}`}
          title="Which App, Account and folder"
          onClick={() => (at ? setAt(null) : open())}
          className="inline-flex min-w-6 shrink-0 items-center justify-center self-stretch rounded-r-md border-l border-hairline px-1 outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
          data-testid="session-account-menu"
        >
          <IconChevronDown width={12} height={12} aria-hidden="true" className="shrink-0 text-muted" />
        </button>
      </span>
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
