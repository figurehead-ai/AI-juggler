/**
 * Settings (AJ-E1203; AJ-E2110, Apple's System Settings pattern): a
 * sidebar of panes — General, Desktop apps, Accounts, Sessions,
 * Switching, Agent access (AJ-E1702), Notifications, Privacy, Your Macs
 * (once one is paired), Keyboard and Advanced — each with grouped
 * sections; a search atop the sidebar finds any of them (AJ-E2905). ⌘, opens the pane viewed last;
 * a link from elsewhere opens its pane at its section (`panes.ts` maps
 * every older section id). Each pane loads when it's first opened.
 *
 * Copy rule: the desktop path is complete. Every setting says what it
 * does in one line; permissions are offered here, never raised by a
 * click elsewhere.
 */
import { useEffect, useState, type JSX } from "react";
import { fleetPeers } from "../lib/ipc/fleet";
import { lazyNamed } from "../lib/lazy";
import { useResource } from "../lib/use-resource";
import { PaneIcon } from "./settings/pane-icons";
import { lastPane, rememberPane, shownPanes, targetOf, type PaneId } from "./settings/panes";
import { searchSettings } from "./settings/settings-index";
import { SettingsSearch } from "./settings/SettingsSearch";
import { PaneTile } from "./settings/parts";

type PaneProps = { anchor?: string };

const PANE_BODIES: Record<PaneId, (props: PaneProps) => JSX.Element> = {
  general: lazyNamed(() => import("./settings/GeneralPane"), "GeneralPane"),
  "desktop-apps": lazyNamed(() => import("./settings/DesktopAppsPane"), "DesktopAppsPane"),
  "the-floor": lazyNamed(() => import("./settings/FloorPane"), "FloorPane"),
  accounts: lazyNamed(() => import("./settings/AccountsPane"), "AccountsPane"),
  sessions: lazyNamed(() => import("./settings/SessionsPane"), "SessionsPane"),
  switching: lazyNamed(() => import("./settings/SwitchingPane"), "SwitchingPane"),
  "agent-access": lazyNamed(() => import("./settings/AgentAccessPane"), "AgentAccessPane"),
  notifications: lazyNamed(() => import("./settings/NotificationsPane"), "NotificationsPane"),
  privacy: lazyNamed(() => import("./settings/PrivacyPane"), "PrivacyPane"),
  macs: lazyNamed(() => import("./settings/MacsPane"), "MacsPane"),
  keyboard: lazyNamed(() => import("./settings/KeyboardPane"), "KeyboardPane"),
  advanced: lazyNamed(() => import("./settings/AdvancedPane"), "AdvancedPane"),
};

/** How many Macs are paired (the Your Macs pane shows once one is). */
function usePairedMacs(): number | null {
  const peers = useResource("fleet-peers", fleetPeers);
  return peers.data ? peers.data.length : peers.error !== undefined ? 0 : null;
}

export function SettingsScreen({
  focusCloud = false,
  focusSection,
}: {
  /** AJ-E1203: open on Figurehead & Cloud (a Cloud chip, Fleet's offer). */
  focusCloud?: boolean;
  /** D-25: open on this section (a link from another screen). */
  focusSection?: string;
}): JSX.Element {
  const target = targetOf(focusSection, focusCloud);
  const [pane, setPane] = useState<PaneId>(target?.pane ?? lastPane() ?? "general");
  // The link's section, only until another pane is chosen.
  const [anchor, setAnchor] = useState<string | undefined>(target?.anchor);
  // Each link opens its section afresh, even the one opened last time.
  const [jumps, setJumps] = useState(0);
  const pairedMacs = usePairedMacs();
  useEffect(() => rememberPane(pane), [pane]);
  // A link while Settings is open (Privacy's "Last messages…").
  useEffect(() => {
    const open = (event: Event) => {
      const next = targetOf((event as CustomEvent<{ section?: string } | undefined>).detail?.section);
      if (!next) return;
      setPane(next.pane);
      setAnchor(next.anchor);
      setJumps((count) => count + 1);
    };
    window.addEventListener("aij:open-settings", open);
    return () => window.removeEventListener("aij:open-settings", open);
  }, []);
  const panes = shownPanes({ pairedMacs }, pane);
  // AJ-E2905: the search; a pane with nothing that matches dims.
  const [query, setQuery] = useState("");
  const labels = Object.fromEntries(panes.map((entry) => [entry.id, entry.label]));
  const hits = searchSettings(query, labels).filter((hit) => hit.pane in labels);
  const matched = new Set(hits.map((hit) => hit.pane));
  const searching = query.trim() !== "";
  const Body = PANE_BODIES[pane];
  return (
    <div className="mx-auto flex w-full max-w-[1040px] gap-8 pb-10" data-testid="settings-screen">
      <nav aria-label="Settings" className="sticky top-0 w-52 shrink-0 self-start pt-1">
        <h2 className="mb-3 px-2 text-title text-ink">Settings</h2>
        <SettingsSearch
          query={query}
          hits={hits}
          onQuery={setQuery}
          onPick={(hit) => {
            setPane(hit.pane);
            setAnchor(hit.rowAnchor);
            setJumps((count) => count + 1);
          }}
        />
        <ul className="flex flex-col gap-0.5" data-testid="settings-nav">
          {panes.map((entry) => (
            <li key={entry.id}>
              <button
                type="button"
                onClick={() => {
                  setPane(entry.id);
                  setAnchor(undefined);
                }}
                aria-current={pane === entry.id ? "page" : undefined}
                className={
                  "flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-body transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent " +
                  // AJ-E3306 (rc26): the selected item carries the accent
                  // (text and edge); a hover is a lighter plain fill, so a
                  // resting pointer never reads as a second selection.
                  (pane === entry.id
                    ? "bg-accent-soft font-medium text-accent-hi ring-1 ring-inset ring-accent/30"
                    : "text-ink hover:bg-surface-2/60") +
                  (searching && !matched.has(entry.id) ? " opacity-45" : "")
                }
                data-pane={entry.id}
              >
                <PaneTile tone={entry.tone}>
                  <PaneIcon pane={entry.id} />
                </PaneTile>
                {entry.label}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <div className="min-w-0 max-w-[760px] flex-1 pb-10"><Body key={`${pane}:${jumps}`} anchor={anchor} /></div>
    </div>
  );
}
