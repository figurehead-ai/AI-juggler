/**
 * AJ-E2110 — Settings' panes, as data: their order, names, tile colour
 * and icon, when each shows, and where every older link into Settings
 * now lands. Apple's System Settings pattern: a sidebar of panes, each
 * with grouped sections.
 */
import { SETTINGS_INDEX } from "./settings-index";

export type PaneId =
  | "general"
  | "desktop-apps"
  | "the-floor"
  | "accounts"
  | "sessions"
  | "switching"
  | "ai-models"
  | "agent-access"
  | "notifications"
  | "privacy"
  | "macs"
  | "keyboard"
  | "advanced";

/** A group of panes under one sidebar row (Settings › AI › Jobs, Models):
 * its name, tile and icon are the row's, and it opens `opens`. */
export interface PaneGroup {
  id: "ai";
  label: string;
  opens: PaneId;
}

export const GROUPS: Record<PaneGroup["id"], PaneGroup> = {
  // AJ-E3812 (D-131): the AI models and the jobs they do.
  ai: { id: "ai", label: "AI", opens: "ai-models" },
};

export interface PaneSpec {
  id: PaneId;
  label: string;
  /** It sits under a group's row, indented, with no tile of its own. */
  group?: PaneGroup["id"];
  /** When it shows in the sidebar (always, unless it says). */
  shown?: (context: PaneContext) => boolean;
}

export interface PaneContext {
  /** How many Macs are paired with this one (null while unknown). */
  pairedMacs: number | null;
}

export const PANES: readonly PaneSpec[] = [
  { id: "general", label: "General" },
  { id: "desktop-apps", label: "Desktop apps" },
  // AJ-E3511 (D-78): the Floor's few choices.
  { id: "the-floor", label: "The Floor" },
  { id: "accounts", label: "Accounts" },
  { id: "sessions", label: "Sessions" },
  { id: "switching", label: "Switching" },
  // AJ-E1548 #5: the job switches live on Models, beside their model.
  { id: "ai-models", label: "Models", group: "ai" },
  // AJ-E1702, D-28: what agents may do in each pool, and your rules.
  // Always shown: every project has its default pool (D-33), whose level
  // applies too, so these controls are in reach before you make a pool.
  { id: "agent-access", label: "Agent access" },
  { id: "notifications", label: "Notifications" },
  { id: "privacy", label: "Privacy" },
  { id: "macs", label: "Your Macs", shown: ({ pairedMacs }) => (pairedMacs ?? 0) > 0 },
  // AJ-E2905: every shortcut, to change or reset.
  { id: "keyboard", label: "Keyboard" },
  { id: "advanced", label: "Advanced" },
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
  // AJ-E1538: Search the words of every session (its own switch): the
  // section, landing on that row (History's empty search links here).
  search: { pane: "privacy", anchor: "stats?search words every session" },
  diagnostics: { pane: "advanced", anchor: "diagnostics" },
  // AJ-E2027: what AI Juggler does on its own, and when.
  scheduled: { pane: "advanced", anchor: "scheduled" },
  uninstall: { pane: "advanced", anchor: "uninstall" },
  privacy: { pane: "privacy" },
  keyboard: { pane: "keyboard" },
  floor: { pane: "the-floor" },
  "the-floor": { pane: "the-floor" },
  shortcuts: { pane: "keyboard" },
  "go-to": { pane: "desktop-apps", anchor: "go-to" },
  "last-messages": { pane: "desktop-apps", anchor: "last-messages" },
  // 0.3.3: the apps and profiles, with Start and Keep alive.
  "keep-alive": { pane: "desktop-apps", anchor: "keep-alive" },
  // AJ-E2325: the moves and their backups (the low-disk offer opens it).
  moves: { pane: "desktop-apps", anchor: "moves" },
  // 0.2's first Agent access section (one page of Settings).
  agents: { pane: "agent-access" },
  // AJ-E1704: whether sessions get the agent tools, and where they're off.
  "agent-tools": { pane: "agent-access", anchor: "agent-tools" },
  // AJ-E3812: Settings › AI opens Models.
  ai: { pane: "ai-models" },
  models: { pane: "ai-models" },
};

/** Where `section` (an older section id, a pane id, or nothing) lands. */
export function targetOf(section: string | undefined, focusCloud = false): SettingsTarget | null {
  if (focusCloud) return ANCHORS.account!;
  if (!section) return null;
  const known = ANCHORS[section];
  if (known) return known;
  // AJ-E2901: any indexed section (⌘K finds settings too).
  const indexed = SETTINGS_INDEX.find((entry) => entry.anchor === section);
  if (indexed) return { pane: indexed.pane, anchor: indexed.anchor };
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
