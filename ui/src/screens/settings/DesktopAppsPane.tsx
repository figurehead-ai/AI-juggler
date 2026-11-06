/**
 * AJ-E2110 — Settings › Desktop apps: which apps' open sessions show,
 * how Go to it reaches a session (the exact window with Accessibility,
 * the terminal's tab with Automation, the key that opens AI Juggler from
 * anywhere) and the session page's Last messages. Every permission is
 * offered here and nowhere else: a click on Go to it never raises one.
 * 0.3.3: the apps and profiles AI Juggler has seen, each with Start (or
 * Open, while it runs) and Keep alive. AJ-E3203: finding the profiles
 * that aren't running, asked on an App's page, is turned off here.
 */
import { useEffect, useId, useState, type JSX } from "react";
import { Button, Chip } from "../../components/primitives";
import { appSettingGet, appSettingSet, osAccessList, osAccessOpenSettings, osAccessRequest } from "../../lib/ipc/settings";
import { errorText } from "../../lib/bridge";
import type { OsAccessInfo } from "../../types/generated/OsAccessInfo";
import { useConversations } from "../../lib/conversations";
import { appsFound, foundWords, setAppShown, useHiddenApps } from "../../lib/desktop-apps";
import { setShowLastMessages, showLastMessages } from "../../lib/last-messages";
import { accessCard, watchAccess } from "../../lib/osAccess";
import { AppIcon } from "../../components/AppIcon";
import { MenuItem, RowMenu } from "../../components/RowMenu";
import { IconMore } from "../../components/icons-canvas";
import { IconRemove } from "../../components/icons-actions";
import { showToast } from "../../components/Toast";
import { showGoTo } from "../../lib/go-to";
import { onPlacesChanged, placesForget, placesKeepAlive, placesOfferAnswer, placesOffers, placesOpen, type KnownPlaceInfo } from "../../lib/ipc/places";
import type { PlaceOfferInfo } from "../../types/generated/PlaceOfferInfo";
import { placeWords, useKnownPlaces } from "../../lib/places";
import { useFindProfiles } from "../../lib/find-profiles";
import { appName, desktopProfile } from "../../lib/app-name";
import { knownAppWord } from "../../lib/rail-apps";
import { Pane, PaneSection, RowNote, SettingRow, Switch, SwitchRow } from "./parts";
import { GoToKeyControl, GoToKeyError, useGoToKey } from "./KeyCapture";

/** AJ-E2102: Go to it may select a session's tab in Terminal or iTerm;
 * off until turned on here. */
export const TERMINAL_TABS_KEY = "go_to_terminal_tabs";

function ShowSessionsFrom(): JSX.Element {
  const { all } = useConversations();
  const found = appsFound(all);
  const hidden = useHiddenApps(found.map((entry) => entry.app.id));
  const [error, setError] = useState<string | null>(null);
  return (
    <PaneSection
      id="show-from"
      title="Show sessions from"
      footer="AI Juggler reads which sessions each app has open from the tools' own files. It never types into them."
    >
      {found.map((entry) => (
        <SwitchRow
          key={entry.app.id}
          title={entry.app.name}
          description={foundWords(entry)}
          // D-38: the app's own icon, as everywhere an app is named.
          icon={<AppIcon appId={entry.app.id} name={entry.app.name} size={24} />}
          checked={!hidden.has(entry.app.id)}
          testId={`desktop-app-${entry.app.id}`}
          onChange={(on) => {
            setError(null);
            void setAppShown(entry.app.id, on).catch((cause) => setError(errorText(cause)));
          }}
        />
      ))}
      {error && (
        <li className="px-4 py-2 text-xs text-danger" role="alert">
          {error}
        </li>
      )}
    </PaneSection>
  );
}

/** The exact window: Accessibility (and System Events), asked for here. */
function ExactWindowRow(): JSX.Element {
  const [entry, setEntry] = useState<OsAccessInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const load = () => void osAccessList().then((list) => setEntry(list.find((item) => item.id === "focus") ?? null), () => null);
    load();
    return watchAccess(load);
  }, []);
  const card = entry ? accessCard(entry) : null;
  const act = () => {
    if (!entry || !card?.primary) return;
    setError(null);
    const step = card.primary.action === "request" ? osAccessRequest(entry.id) : osAccessOpenSettings(entry.id).then(() => osAccessList());
    void step.then((list) => setEntry(list.find((item) => item.id === "focus") ?? null), (cause) => setError(errorText(cause)));
  };
  return (
    <SettingRow
      title="Bring the exact window to the front"
      description="Needs Accessibility. Without it, Go to it brings the right app forward, and you pick the window."
      testId="desktop-exact-window"
      control={
        entry === null ? (
          <Chip tone="neutral">Not on this computer</Chip>
        ) : card?.state === "allowed" ? (
          <Chip tone="success">On</Chip>
        ) : (
          <Button size="sm" variant="secondary" onClick={act} data-testid="desktop-exact-window-setup">
            {card?.primary?.action === "request" ? "Set up…" : "Open System Settings"}
          </Button>
        )
      }
    >
      {error && (
        <span className="text-xs text-danger" role="alert">
          {error}
        </span>
      )}
    </SettingRow>
  );
}

function TerminalTabsRow(): JSX.Element | null {
  const [on, setOn] = useState<boolean | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    void appSettingGet(TERMINAL_TABS_KEY).then((value) => setOn(value === "1"), () => setOn(false));
  }, []);
  if (on === null) return null;
  return (
    <SwitchRow
      title="Select the right terminal tab"
      description="Needs Automation: macOS asks once per terminal whether AI Juggler may control it. Off, the terminal comes forward and you pick the tab."
      checked={on}
      note={note}
      testId="desktop-terminal-tabs"
      onChange={(next) => {
        setNote(null);
        void appSettingSet(TERMINAL_TABS_KEY, next ? "1" : "").then(
          () => {
            setOn(next);
            setNote(next ? "Go to it selects the session's tab." : "Go to it brings the terminal forward only.");
          },
          (cause) => setNote(errorText(cause)),
        );
      }}
    />
  );
}

/** The key that opens AI Juggler from any app. None until chosen. The
 * control is shared with Settings › Keyboard (AJ-E2905). */
function HotkeyRow(): JSX.Element {
  const hotkey = useGoToKey();
  return (
    <SettingRow
      title="Open AI Juggler from anywhere"
      description={
        hotkey.binding
          ? "Press it in any app to bring AI Juggler up on Go to."
          : "No key yet: choose one. ⌥Space isn't offered, because ChatGPT uses it."
      }
      testId="desktop-hotkey"
      control={<GoToKeyControl hotkey={hotkey} prefix="desktop-hotkey" />}
    >
      <GoToKeyError hotkey={hotkey} prefix="desktop-hotkey" />
    </SettingRow>
  );
}

/** One app or profile: what it is now, Start (Open while it runs), and
 * Keep alive. */
export function PlaceRow({ place, name: named }: { place: KnownPlaceInfo; name?: string }): JSX.Element {
  // D-62: named as its App ("Claude app · Work Max"), or as you named it.
  const name = named ?? appName(knownAppWord(place), desktopProfile(place.instance, place.profile));
  const id = useId();
  const [error, setError] = useState<string | null>(null);
  const running = place.app_pid !== null;
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);
  const open = () => {
    setError(null);
    showToast({ id: "go-to", message: running ? `Bringing ${name} forward…` : `Starting ${name}…` });
    placesOpen(place.id).then(
      showGoTo,
      (cause) => setError(errorText(cause)),
    );
  };
  return (
    <SettingRow
      title={name}
      titleId={`${id}-title`}
      description={placeWords(place)}
      icon={<AppIcon appId={place.app_id} appPid={place.app_pid} name={place.app} size={24} />}
      testId={`desktop-place-${place.app_id}${place.instance ? `-${place.instance}` : ""}`}
      control={
        <>
          <Button size="sm" variant="secondary" onClick={open} disabled={!running && place.cannot_start !== null} title={running ? undefined : (place.cannot_start ?? undefined)} aria-label={`${running ? "Open" : "Start"} ${name}`} data-testid="desktop-place-open">
            {running ? "Open" : "Start"}
          </Button>
          <span id={`${id}-keep`} className="ml-1 text-xs font-medium text-muted">
            Keep alive
          </span>
          <Switch
            checked={place.keep_alive}
            disabled={!place.keep_alive && place.cannot_start !== null}
            labelledBy={`${id}-keep ${id}-title`}
            testId="desktop-place-keep"
            onChange={(on) => {
              setError(null);
              void placesKeepAlive(place.id, on).catch((cause) => setError(errorText(cause)));
            }}
          />
          {/* Forget, in its ⋯ (design review #65: a lone Forget broke the
              column). A profile found by its folder comes back while
              finding is on: its folder, or Find profiles, decides. */}
          {!running && !place.keep_alive && !place.found && (
            <>
              <button
                type="button"
                onClick={(event) => {
                  const box = event.currentTarget.getBoundingClientRect();
                  setMenuAt({ x: box.left, y: box.bottom + 4 });
                }}
                aria-label={`More for ${name}`}
                aria-haspopup="menu"
                className="flex size-7 items-center justify-center rounded-md text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
                data-testid="desktop-place-more"
              >
                <IconMore width={16} height={16} />
              </button>
              {menuAt && (
                <RowMenu heading={name} ariaLabel={`${name} actions`} at={menuAt} onClose={() => setMenuAt(null)}>
                  <MenuItem danger onClick={() => (setMenuAt(null), void placesForget(place.id).catch((cause) => setError(errorText(cause))))} testId="desktop-place-forget">
                    <span className="flex items-center gap-2">
                      <IconRemove width={15} height={15} aria-hidden="true" />
                      Forget
                    </span>
                  </MenuItem>
                </RowMenu>
              )}
            </>
          )}
        </>
      }
    >
      {place.paused && <RowNote tone="warning">Open it to start it and keep it alive again.</RowNote>}
      {place.cannot_start && <RowNote tone="warning">{`Can't be started from here: ${place.cannot_start}.`}</RowNote>}
      {error && <RowNote tone="warning">{error}</RowNote>}
    </SettingRow>
  );
}

/** AJ-E3203: finding the profiles that aren't running, by the names of
 * the folders beside one that ran. Shown once there's a folder to read,
 * or it's on. */
function FindProfilesRow(): JSX.Element | null {
  const { info, answer } = useFindProfiles();
  const [note, setNote] = useState<string | null>(null);
  if (info === null || (info.folders.length === 0 && info.answer !== "on")) return null;
  const where = info.folders.map((folder) => folder.folder).join(", ");
  return (
    <SwitchRow
      title="Find profiles that aren't running"
      description={
        <>
          Reads the names of the folders beside a profile that ran{where ? <> (<span className="break-all font-mono">{where}</span>)</> : null}, and nothing inside them. Off forgets the profiles found that way.
        </>
      }
      checked={info.answer === "on"}
      note={note}
      testId="desktop-find-profiles"
      onChange={(next) => {
        setNote(null);
        answer(next).catch((cause) => setNote(errorText(cause)));
      }}
    />
  );
}

/** D-84: apps seen running that the user hasn't added: offered, never
 * added by themselves. Add makes one an App; Not now stops offering it. */
function FoundOnThisMac(): JSX.Element | null {
  const [offers, setOffers] = useState<PlaceOfferInfo[] | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    const load = () => void placesOffers().then(setOffers, () => setOffers([]));
    load();
    return onPlacesChanged(load);
  }, []);
  if (!offers || offers.length === 0) return null;
  const answer = (offer: PlaceOfferInfo, add: boolean) => {
    setNote(null);
    placesOfferAnswer(offer.id, add).then(
      () => setOffers((now) => (now ?? []).filter((one) => one.id !== offer.id)),
      (cause) => setNote(errorText(cause)),
    );
  };
  return (
    <PaneSection id="found" title="Found on this Mac" footer="Seen running, not added. AI Juggler adds an app only when you say so; until then it isn't in your apps, the rail or Home.">
      {offers.map((offer) => (
        <li key={offer.id} className="flex flex-wrap items-center gap-3 px-4 py-2.5" data-testid="desktop-offer">
          <span className="min-w-0 flex-1 truncate text-sm text-ink">{offer.label}</span>
          <Button size="sm" variant="secondary" onClick={() => answer(offer, true)} data-testid="desktop-offer-add">Add</Button>
          <Button size="sm" variant="ghost" onClick={() => answer(offer, false)} data-testid="desktop-offer-not-now">Not now</Button>
        </li>
      ))}
      {note && <RowNote>{note}</RowNote>}
    </PaneSection>
  );
}

/** 0.3.3: the apps and profiles seen running, with Start and Keep alive. */
function YourApps(): JSX.Element | null {
  const places = useKnownPlaces();
  if (places === null) return null;
  return (
    <PaneSection
      id="keep-alive"
      title="Your apps"
      footer="Start opens an app that isn't running, in its own profile. Keep alive starts it again whenever it stops (a crash, ⌘Q or Force Quit), behind the app you're in, at most 3 times in 10 minutes. Never while the screen is locked, never on another account; a stop in AI Juggler pauses it until you open it."
    >
      {places.length === 0 ? (
        <li className="px-4 py-3 text-xs text-muted" data-testid="desktop-places-empty">
          Apps you add show here: an app you import, or one found on this Mac that you add.
        </li>
      ) : (
        places.map((place) => <PlaceRow key={place.id} place={place} />)
      )}
      <FindProfilesRow />
    </PaneSection>
  );
}

function LastMessages(): JSX.Element {
  const [on, setOn] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void showLastMessages().then(setOn, () => setOn(true));
  }, []);
  return (
    <PaneSection
      id="last-messages"
      title="Last messages"
      footer="Read from the session's transcript on this computer when you open it, and never sent. Only its subagents' prompts and results are kept, in the local cache."
    >
      {on !== null && (
        <SwitchRow
          title="Show a session's last messages"
          description="Its page shows the last few turns, with secrets taken out. Never in the rail, the menu bar or notifications. Claude app chats show their title only."
          checked={on}
          testId="desktop-last-messages"
          note={error}
          onChange={(next) => {
            setError(null);
            void setShowLastMessages(next).then(() => setOn(next), (cause) => setError(errorText(cause)));
          }}
        />
      )}
    </PaneSection>
  );
}

export function DesktopAppsPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="desktop-apps" title="Desktop apps" anchor={anchor}>
      <ShowSessionsFrom />
      <FoundOnThisMac />
      <YourApps />
      <PaneSection
        id="go-to"
        title="Going to a session"
        footer="Go to it always brings the session's app forward. These go further, and only once you turn them on here."
      >
        <ExactWindowRow />
        <TerminalTabsRow />
        <HotkeyRow />
      </PaneSection>
      <LastMessages />
    </Pane>
  );
}
