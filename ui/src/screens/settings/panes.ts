/**
 * AJ-E2110 — Settings' panes, as data: their order, names, tile colour
 * and icon, when each shows, and where every older link into Settings
 * now lands. Apple's System Settings pattern: a sidebar of panes, each
 * with grouped sections.
 */

export type PaneId =
  | "general"
  | "desktop-apps"
  | "accounts"
  | "sessions"
  | "switching"
  | "agent-access"
  | "notifications"
  | "privacy"
  | "macs"
  | "advanced";

/** The tile's colour: a token from styles.css, never a hex. */
export type PaneTone = "muted" | "accent" | "live" | "success" | "warning" | "danger" | "faint";

export interface PaneSpec {
  id: PaneId;
  label: string;
  tone: PaneTone;
  /** When it shows in the sidebar (always, unless it says). */
  shown?: (context: PaneContext) => boolean;
}

export interface PaneContext {
  /** How many Macs are paired with this one (null while unknown). */
  pairedMacs: number | null;
}

export const PANES: readonly PaneSpec[] = [
  { id: "general", label: "General", tone: "muted" },
  { id: "desktop-apps", label: "Desktop apps", tone: "accent" },
  { id: "accounts", label: "Accounts", tone: "live" },
  { id: "sessions", label: "Sessions", tone: "success" },
  { id: "switching", label: "Switching", tone: "accent" },
  // AJ-E1702, D-28: what agents may do in each pool, and your rules.
  // Always shown: every project has its default pool (D-33), whose level
  // applies too, so these controls are in reach before you make a pool.
  { id: "agent-access", label: "Agent access", tone: "warning" },
  { id: "notifications", label: "Notifications", tone: "danger" },
  { id: "privacy", label: "Privacy", tone: "live" },
  { id: "macs", label: "Your Macs", tone: "success", shown: ({ pairedMacs }) => (pairedMacs ?? 0) > 0 },
  { id: "advanced", label: "Advanced", tone: "faint" },
];

/** Where a link into Settings lands: a pane, and a section in it. */
export interface SettingsTarget {
  pane: PaneId;
  anchor?: string;
}

/** Every section id a link has ever opened (D-25's and AJ-E1010's), and
 * the panes' own ids, with where each lives now. */
const ANCHORS: Record<string, SettingsTarget> = {
  general: { pane: "general" },
  updates: { pane: "general", anchor: "about" },
  account: { pane: "advanced", anchor: "account" },
  cloud: { pane: "advanced", anchor: "account" },
  packs: { pane: "switching", anchor: "packs" },
  sessions: { pane: "sessions" },
  cleanup: { pane: "advanced", anchor: "cleanup" },
  renewal: { pane: "accounts", anchor: "renewal" },
  tools: { pane: "accounts", anchor: "tools" },
  notifications: { pane: "notifications" },
  fleet: { pane: "macs", anchor: "fleet" },
  data: { pane: "advanced", anchor: "data" },
  // ADR-0023: what AI Juggler keeps on this Mac to be quick.
  "local-cache": { pane: "advanced", anchor: "local-cache" },
  // AJ-E1538: Search the words of every session (its own switch).
  search: { pane: "privacy", anchor: "stats" },
  diagnostics: { pane: "advanced", anchor: "diagnostics" },
  uninstall: { pane: "advanced", anchor: "uninstall" },
  privacy: { pane: "privacy" },
  "go-to": { pane: "desktop-apps", anchor: "go-to" },
  "last-messages": { pane: "desktop-apps", anchor: "last-messages" },
  // 0.3.3: the apps and profiles, with Start and Keep alive.
  "keep-alive": { pane: "desktop-apps", anchor: "keep-alive" },
  // 0.2's first Agent access section (one page of Settings).
  agents: { pane: "agent-access" },
  // AJ-E1704: whether sessions get the agent tools, and where they're off.
  "agent-tools": { pane: "agent-access", anchor: "agent-tools" },
};

/** Where `section` (an older section id, a pane id, or nothing) lands. */
export function targetOf(section: string | undefined, focusCloud = false): SettingsTarget | null {
  if (focusCloud) return ANCHORS.account!;
  if (!section) return null;
  const known = ANCHORS[section];
  if (known) return known;
  const pane = PANES.find((entry) => entry.id === section);
  return pane ? { pane: pane.id } : null;
}

/** The panes the sidebar lists now. A pane a link opened shows too,
 * so the user sees where they are. */
export function shownPanes(context: PaneContext, current: PaneId): PaneSpec[] {
  return PANES.filter((pane) => pane.id === current || (pane.shown?.(context) ?? true));
}

const LAST_PANE = "aij.settings.pane";

/** The pane viewed last (this Mac's window only), when it still exists. */
export function lastPane(): PaneId | null {
  try {
    const stored = localStorage.getItem(LAST_PANE);
    return PANES.find((pane) => pane.id === stored && pane.shown === undefined)?.id ?? null;
  } catch {
    return null;
  }
}

export function rememberPane(pane: PaneId): void {
  try {
    localStorage.setItem(LAST_PANE, pane);
  } catch {
    // Storage off: Settings opens on General next time.
  }
}
