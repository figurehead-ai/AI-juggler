/**
 * Settings (AJ-E1203; AJ-E2110, Apple's System Settings pattern): a
 * sidebar of panes — General, Desktop apps, Accounts, Sessions,
 * Switching, Agent access (AJ-E1702), Notifications, Privacy, Your Macs
 * (once one is paired) and Advanced — each with grouped sections. ⌘, opens the pane viewed last;
 * a link from elsewhere opens its pane at its section (`panes.ts` maps
 * every older section id). Each pane loads when it's first opened.
 *
 * Copy rule: the desktop path is complete. Every setting says what it
 * does in one line; permissions are offered here, never raised by a
 * click elsewhere.
 */
import { useEffect, useState, type JSX } from "react";
import { fleetPeers } from "../lib/bridge";
import { lazyNamed } from "../lib/lazy";
import { PaneIcon } from "./settings/pane-icons";
import { lastPane, rememberPane, shownPanes, targetOf, type PaneId } from "./settings/panes";
import { PaneTile } from "./settings/parts";

type PaneProps = { anchor?: string };

const PANE_BODIES: Record<PaneId, (props: PaneProps) => JSX.Element> = {
  general: lazyNamed(() => import("./settings/GeneralPane"), "GeneralPane"),
  "desktop-apps": lazyNamed(() => import("./settings/DesktopAppsPane"), "DesktopAppsPane"),
  accounts: lazyNamed(() => import("./settings/AccountsPane"), "AccountsPane"),
  sessions: lazyNamed(() => import("./settings/SessionsPane"), "SessionsPane"),
  switching: lazyNamed(() => import("./settings/SwitchingPane"), "SwitchingPane"),
  "agent-access": lazyNamed(() => import("./settings/AgentAccessPane"), "AgentAccessPane"),
  notifications: lazyNamed(() => import("./settings/NotificationsPane"), "NotificationsPane"),
  privacy: lazyNamed(() => import("./settings/PrivacyPane"), "PrivacyPane"),
  macs: lazyNamed(() => import("./settings/MacsPane"), "MacsPane"),
  advanced: lazyNamed(() => import("./settings/AdvancedPane"), "AdvancedPane"),
};

/** How many Macs are paired (the Your Macs pane shows once one is). */
function usePairedMacs(): number | null {
  const [count, setCount] = useState<number | null>(null);
  useEffect(() => {
    void fleetPeers().then((peers) => setCount(peers.length), () => setCount(0));
  }, []);
  return count;
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
  const Body = PANE_BODIES[pane];
  return (
    <div className="mx-auto flex w-full max-w-5xl gap-8" data-testid="settings-screen">
      <nav aria-label="Settings" className="sticky top-0 w-52 shrink-0 self-start pt-1">
        <h2 className="mb-3 px-2 text-lg font-semibold tracking-tight">Settings</h2>
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
                  "flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-sm transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent " +
                  (pane === entry.id ? "bg-accent-soft font-medium text-ink" : "text-ink hover:bg-surface-2")
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
