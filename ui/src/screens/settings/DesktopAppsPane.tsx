/**
 * AJ-E2110 — Settings › Desktop apps: which apps' open sessions show,
 * how Go to it reaches a session (the exact window with Accessibility,
 * the terminal's tab with Automation, the key that opens AI Juggler from
 * anywhere) and the session page's Last messages. Every permission is
 * offered here and nowhere else: a click on Go to it never raises one.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Chip } from "../../components/primitives";
import { appSettingGet, appSettingSet, errorText, goToHotkeySet, osAccessList, osAccessOpenSettings, osAccessRequest, type OsAccessInfo } from "../../lib/bridge";
import { useConversations } from "../../lib/conversations";
import { appsFound, foundWords, setAppShown, useHiddenApps } from "../../lib/desktop-apps";
import { hotkeyRefusal, hotkeyWords, toAccelerator } from "../../lib/hotkey";
import { setShowLastMessages, showLastMessages } from "../../lib/last-messages";
import { accessCard, watchAccess } from "../../lib/osAccess";
import { GO_TO_HOTKEY_EVENT } from "../../lib/outside";
import { AppIcon } from "../../components/AppIcon";
import { Pane, PaneSection, SettingRow, SwitchRow } from "./parts";

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
      footer="AI Juggler reads which sessions each app has open from the apps' own files. It never types into them."
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
      description="Needs Accessibility. Without it, Go to it brings the right app and profile forward, and you pick the window."
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

/** The key that opens AI Juggler from any app. None until chosen. */
function HotkeyRow(): JSX.Element {
  const [binding, setBinding] = useState<string | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void appSettingGet("go_to_hotkey").then((stored) => setBinding(stored ?? ""), () => setBinding(""));
  }, []);
  const save = (next: string) => {
    setError(null);
    void goToHotkeySet(next).then(
      () => {
        setBinding(next);
        setCapturing(false);
        window.dispatchEvent(new CustomEvent(GO_TO_HOTKEY_EVENT, { detail: next }));
      },
      (cause) => setError(errorText(cause)),
    );
  };
  return (
    <SettingRow
      title="Open AI Juggler from anywhere"
      description={
        binding
          ? "Press it in any app to bring AI Juggler up on Go to."
          : "No key yet: choose one. ⌥Space isn't offered, because ChatGPT uses it."
      }
      testId="desktop-hotkey"
      control={
        capturing ? (
          <button
            type="button"
            autoFocus
            className="rounded-lg border border-accent bg-accent-soft px-2.5 py-1 text-xs text-ink"
            onKeyDown={(event) => {
              event.preventDefault();
              if (event.key === "Escape") {
                setCapturing(false);
                return;
              }
              const combo = toAccelerator(event.nativeEvent);
              if (combo === null) return;
              const refusal = hotkeyRefusal(combo);
              if (refusal) setError(refusal);
              else save(combo);
            }}
            onBlur={() => setCapturing(false)}
            data-testid="desktop-hotkey-capture"
          >
            Press the keys… <span className="text-muted">(Esc cancels)</span>
          </button>
        ) : (
          <>
            {binding ? (
              <kbd className="rounded-md border border-hairline bg-surface-2 px-2 py-0.5 font-mono text-xs text-ink" data-testid="desktop-hotkey-key">
                {hotkeyWords(binding)}
              </kbd>
            ) : (
              <span className="text-xs text-muted">None</span>
            )}
            <Button size="sm" variant="secondary" onClick={() => setCapturing(true)} data-testid="desktop-hotkey-choose">
              {binding ? "Change…" : "Choose…"}
            </Button>
            {binding && (
              <Button size="sm" variant="ghost" onClick={() => save("")} data-testid="desktop-hotkey-clear">
                Turn off
              </Button>
            )}
          </>
        )
      }
    >
      {error && (
        <span className="text-xs text-danger" role="alert" data-testid="desktop-hotkey-error">
          {error}
        </span>
      )}
    </SettingRow>
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
      <PaneSection
        id="go-to"
        title="Going to a session"
        footer="Go to it always brings the session's app and profile forward. These go further, and only once you turn them on here."
      >
        <ExactWindowRow />
        <TerminalTabsRow />
        <HotkeyRow />
      </PaneSection>
      <LastMessages />
    </Pane>
  );
}
