/**
 * AJ-E2905: what Settings' search finds, kept beside `PANES` so no lazy
 * pane is drawn to search it: every section, by its pane and anchor (its
 * `PaneSection` id), its title, and the words of its settings, with a few
 * other words people use for them ("mute" finds Notifications › Muted). A
 * test draws every pane and fails when a section isn't here or an anchor
 * is gone, so the index can't drift.
 */
import type { PaneId } from "./panes";
import { SHORTCUT_AREAS, SHORTCUTS } from "../../lib/shortcuts";

/** A Keyboard area's section id: the first is the pane's own anchor. */
export function keyboardSectionId(area: string, index: number): string {
  return index === 0 ? "keyboard" : `keyboard-${area.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")}`;
}

export interface IndexEntry {
  pane: PaneId;
  /** The section's id (`PaneSection id`), its anchor. */
  anchor: string;
  title: string;
  /** Its settings' labels and hints, and other words for them. */
  words: string;
}

export const SETTINGS_INDEX: readonly IndexEntry[] = [
  { pane: "general", anchor: "appearance", title: "Appearance", words: "theme light dark match the system density compact comfortable text size colours" },
  { pane: "general", anchor: "editor", title: "Open projects in", words: "editor finder vs code cursor zed open folder" },
  { pane: "general", anchor: "about", title: "About & updates", words: "version update channel stable beta nightly go back earlier version release notes check for updates" },
  { pane: "desktop-apps", anchor: "show-from", title: "Show sessions from", words: "desktop apps claude app codex app chatgpt hide app show sessions" },
  { pane: "desktop-apps", anchor: "keep-alive", title: "Your apps", words: "start open keep alive restart profile app relaunch crash" },
  { pane: "desktop-apps", anchor: "last-messages", title: "Last messages", words: "last message preview words read conversations" },
  { pane: "desktop-apps", anchor: "go-to", title: "Going to a session", words: "go to hotkey global key from any app keyboard shortcut bring forward" },
  { pane: "the-floor", anchor: "floor-opening", title: "Opening", words: "the floor office open when ai juggler starts keep it on top window" },
  { pane: "the-floor", anchor: "floor-office", title: "The office", words: "the floor apps per floor motion calm full off decay dust reset the view re-create recreate office look layout" },
  { pane: "accounts", anchor: "accounts-page", title: "Accounts and apps", words: "subscription account app profile sign in plan" },
  { pane: "accounts", anchor: "renewal", title: "Renewal dates", words: "renewal billing date renews subscription reminder" },
  { pane: "accounts", anchor: "tools", title: "Tools on this computer", words: "claude code codex cli installed version update tool" },
  { pane: "sessions", anchor: "sessions", title: "Sessions AI Juggler starts", words: "launch settings terminal driven model permissions start restart crashed keep awake" },
  { pane: "switching", anchor: "packs", title: "Handovers and keys", words: "switch continue in another app handover keys secrets passwords hide strip notes" },
  { pane: "agent-access", anchor: "agent-pools", title: "Pools", words: "pool access level agents messages tasks seal" },
  { pane: "agent-access", anchor: "agent-rules", title: "Rules you made", words: "rules pool allow deny agent" },
  { pane: "agent-access", anchor: "agent-tools", title: "Agent tools in sessions", words: "mcp agent tools bridge messages tasks off" },
  { pane: "notifications", anchor: "notifications", title: "Alerts and the digest", words: "notify notification quiet hours digest waiting threshold quota warning 80 95 send test sound alert" },
  { pane: "notifications", anchor: "notification-mutes", title: "Muted", words: "mute muted unmute silence project kind snooze" },
  { pane: "privacy", anchor: "reads", title: "What AI Juggler reads", words: "privacy reads files transcripts what is read trust" },
  { pane: "privacy", anchor: "stats", title: "Usage statistics, crash reports and search", words: "usage statistics telemetry crash reports search index words send share" },
  { pane: "macs", anchor: "fleet", title: "What your other Macs may do here", words: "fleet paired macs other computers access stop start" },
  { pane: "advanced", anchor: "cleanup", title: "Cleanup", words: "cleanup left behind processes dev server stop automatic tidy" },
  { pane: "advanced", anchor: "data", title: "Data: storage & retention", words: "storage retention history keep days database size" },
  { pane: "advanced", anchor: "backup", title: "Data: backup & restore", words: "backup restore export import file move to another mac" },
  { pane: "advanced", anchor: "local-cache", title: "Local cache", words: "cache clear size limit disk speed" },
  { pane: "advanced", anchor: "diagnostics", title: "Diagnostics", words: "diagnostics export zip logs report a bug problem" },
  { pane: "advanced", anchor: "debug-mode", title: "Debug mode", words: "debug log switches launches steps clear the log" },
  { pane: "advanced", anchor: "account", title: "Figurehead & Cloud", words: "figurehead account cloud backup sign in link device" },
  { pane: "advanced", anchor: "uninstall", title: "Before you uninstall", words: "uninstall remove delete prepare" },
  // Keyboard: one section per area, found by its shortcuts' names too.
  ...SHORTCUT_AREAS.map((area, index) => ({
    pane: "keyboard" as const,
    anchor: keyboardSectionId(area, index),
    title: area,
    words: `shortcuts keys keyboard rebind change reset ${SHORTCUTS.filter((entry) => entry.area === area)
      .map((entry) => entry.label)
      .join(" ")}`,
  })),
];

/** The words of `query`, folded. */
export function folded(text: string): string[] {
  return text
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .split(/[^a-z0-9%]+/)
    .filter(Boolean);
}

/** A search hit's anchor: its section and the words that found it
 * ("notifications?quiet hours"), so its pane lands on the row they name,
 * not only the section (design review 25). A link's anchor is the section
 * alone. */
export function searchAnchor(section: string, query: string): string {
  const words = folded(query);
  return words.length === 0 ? section : `${section}?${words.join(" ")}`;
}

/** An anchor's section, and the searched words it carries (if any). */
export function splitAnchor(anchor: string): { section: string; words: string[] } {
  const at = anchor.indexOf("?");
  return at < 0 ? { section: anchor, words: [] } : { section: anchor.slice(0, at), words: folded(anchor.slice(at + 1)) };
}

/** What counts as one row in a section: a settings row, a list item, a
 * group of controls. */
const ROWS = "[data-settings-row], li, fieldset";

/** The row in `section` that `words` name best: the most of them starting
 * a word of its text, then the smallest such row. Null when none does
 * (the words matched only the index's other words): the section it is. */
export function matchedRow(section: HTMLElement, words: readonly string[]): HTMLElement | null {
  let best: HTMLElement | null = null;
  let bestScore = 0;
  let bestLength = Number.POSITIVE_INFINITY;
  for (const row of section.querySelectorAll<HTMLElement>(ROWS)) {
    const raw = row.textContent ?? "";
    const text = folded(raw);
    const score = words.filter((word) => text.some((t) => t.startsWith(word))).length;
    if (score === 0) continue;
    if (score > bestScore || (score === bestScore && raw.length < bestLength)) {
      best = row;
      bestScore = score;
      bestLength = raw.length;
    }
  }
  return best;
}

/** The sections `query` finds, best first: every word must start a word
 * of the entry; a match in its title ranks above one in its words. Each
 * hit's `anchor` is its section (a link to it, ⌘K's); its `rowAnchor`
 * also carries the query (`searchAnchor`), for a pane to land on the row. */
export function searchSettings(query: string, labels: Record<string, string>): (IndexEntry & { paneLabel: string; rowAnchor: string })[] {
  const wanted = folded(query);
  if (wanted.length === 0) return [];
  const scored = SETTINGS_INDEX.flatMap((entry) => {
    const title = folded(`${labels[entry.pane] ?? ""} ${entry.title}`);
    const words = folded(entry.words);
    let score = 0;
    for (const word of wanted) {
      if (title.some((t) => t.startsWith(word))) score += 2;
      else if (words.some((t) => t.startsWith(word))) score += 1;
      else return [];
    }
    return [{ ...entry, rowAnchor: searchAnchor(entry.anchor, query), paneLabel: labels[entry.pane] ?? entry.pane, score }];
  });
  return scored.sort((a, b) => b.score - a.score).map(({ score: _, ...entry }) => entry);
}
