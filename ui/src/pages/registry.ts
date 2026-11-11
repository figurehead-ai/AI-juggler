/**
 * AJ-E1533 — one page model (ADR-0019 §3: the shell renders screens
 * from a registry). Every item opens the same kind of page: an identity
 * header (its type, mark, name, parent and a few facts), then its tabs,
 * the first being the item's body — a session's terminal, a project's
 * sessions. A tab shows only when it has something to show; the actions
 * live in the header and its ⋯ menu, never in a tab of their own.
 */
import { MACHINE_WORD, THIS_MACHINE_IN_TEXT } from "../lib/machine-words";
import { keep, readKept } from "../lib/local-store";

export type PageKind = "session" | "conversation" | "project" | "app" | "subscription" | "tool" | "machine" | "remote-session" | "pool";

/** The type, in words, on every page's header. A conversation started
 * outside AI Juggler is a session too (it says "outside AI Juggler"). */
export const PAGE_LABELS: Record<PageKind, string> = {
  session: "Session",
  conversation: "Session",
  project: "Project",
  // D-62: an App's own page (AJ-E3204; a folder is its CLI App), and an
  // Account's, a subscription (AJ-E3205).
  app: "App",
  subscription: "Account",
  tool: "Tool",
  machine: MACHINE_WORD,
  "remote-session": "Session",
  pool: "Pool",
};

export type PageTabId =
  | "terminal"
  | "window"
  | "sessions"
  | "overview"
  | "usage"
  | "quality"
  | "processes"
  | "history"
  | "actions"
  | "members"
  | "messages"
  | "tasks"
  | "subagents"
  | "access"
  | "pools"
  | "projects"
  | "settings"
  | "apps"
  | "launch";

export type PageTab = {
  id: PageTabId;
  label: string;
  /** Where its numbers come from, in plain words (the footer). */
  source: string;
  /** 0.3: how many it holds, beside its name ("Sessions 3"), when known. */
  count?: number;
};

const TAB: Record<PageTabId, PageTab> = {
  terminal: { id: "terminal", label: "Terminal", source: "this computer's process monitor" },
  window: { id: "window", label: "Window preview", source: "this computer's process monitor" },
  sessions: { id: "sessions", label: "Sessions", source: "the tools' own session files" },
  overview: { id: "overview", label: "Overview", source: "this computer's process monitor" },
  usage: { id: "usage", label: "Usage", source: "usage tracking" },
  quality: { id: "quality", label: "Quality", source: "this computer's process monitor" },
  processes: { id: "processes", label: "Processes", source: "this computer's process monitor" },
  history: { id: "history", label: "History", source: "AI Juggler's record of the session" },
  actions: { id: "actions", label: "Actions", source: "the paired Mac" },
  members: { id: "members", label: "Members", source: "AI Juggler's pools and the sessions' own transcripts" },
  messages: { id: "messages", label: "Messages", source: "AI Juggler's own messages, on this computer" },
  tasks: { id: "tasks", label: "Tasks", source: "the pool's tasks, kept by AI Juggler on this computer" },
  subagents: { id: "subagents", label: "Subagents", source: "the tool's own records of its subagents, read-only" },
  access: { id: "access", label: "Agent access", source: "your choices for this pool" },
  // AJ-E3104: the pools its sessions are in (a project's own and yours).
  pools: { id: "pools", label: "Pools", source: "AI Juggler's pools, kept on this computer" },
  projects: { id: "projects", label: "Projects", source: "the tools' own session files" },
  settings: { id: "settings", label: "Settings", source: "your choices for this app" },
  apps: { id: "apps", label: "Apps", source: "AI Juggler's apps and their sign-ins" },
  // AJ-E3204 R2: a CLI App's launch settings, the sets it starts with.
  launch: { id: "launch", label: "Launch settings", source: "the launch settings you saved for this app" },
};

/** What decides which tabs a page has: only what it has to show. */
export type TabFacts = {
  /** A session in a desktop app's window rather than a terminal. */
  desktop?: boolean;
  /** A session with recorded events (resumes, handovers, crashes). */
  hasHistory?: boolean;
  /** Anything measured to show on Usage. */
  hasUsage?: boolean;
  /** An account's past sessions. */
  hasSessions?: boolean;
  /** E18: a session with messages to or from it. */
  hasMessages?: boolean;
  /** E19: a session that holds or worked a task. */
  hasTasks?: boolean;
  /** AJ-E2404: another app's session, open now (its processes show). */
  live?: boolean;
  /** AJ-E3204: a CLI App (its launch settings are its own). */
  cli?: boolean;
  /** AJ-E1520: a session whose tool records subagents, and it started
   * some. */
  hasSubagents?: boolean;
  /** AJ-E3104: this Mac's page (a paired Mac's has its peer's tabs). */
  local?: boolean;
  /** 0.3: how many each tab holds, where known (a project's sessions,
   * tasks, subagents). */
  counts?: Partial<Record<PageTabId, number>>;
};

/** Where an Overview's numbers come from, by page (P21: never one
 * source named for all). */
const OVERVIEW_SOURCE: Partial<Record<PageKind, string>> = {
  project: "AI Juggler's projects and the tools' session files",
  app: "the apps' own session files and usage tracking",
  subscription: "its apps' sign-ins, limits and usage tracking",
  tool: "the tool's install and its apps",
  // AJ-E2108: where it runs, and its last messages from its transcript's end.
  session: "the apps' own session files and the end of its transcript",
  conversation: "the apps' own session files and the end of its transcript",
};

/** The page's tabs, its body first. */
export function pageTabs(kind: PageKind, facts: TabFacts = {}): PageTab[] {
  const source = OVERVIEW_SOURCE[kind];
  const tabs = tabsOf(kind, facts).map((tab) => (facts.counts?.[tab.id] !== undefined ? { ...tab, count: facts.counts[tab.id] } : tab));
  return source ? tabs.map((tab) => (tab.id === "overview" ? { ...tab, source } : tab)) : tabs;
}

function tabsOf(kind: PageKind, facts: TabFacts): PageTab[] {
  const when = (shown: boolean | undefined, tab: PageTabId) => (shown ? [TAB[tab]] : []);
  switch (kind) {
    // AJ-E2108: its body first. A terminal's session: its terminal IS its
    // Overview (the conversation, with the side column beside it, as on
    // every session's page); a desktop app's: its window, then Overview.
    case "session":
      // E18: its Messages, once it has any.
      return facts.desktop
        ? [TAB.window, TAB.overview, ...when(facts.hasSubagents, "subagents"), ...when(facts.hasMessages, "messages"), ...when(facts.hasTasks, "tasks"), TAB.pools, TAB.usage, TAB.processes, ...when(facts.hasHistory, "history")]
        : [{ ...TAB.terminal, label: "Overview" }, ...when(facts.hasSubagents, "subagents"), ...when(facts.hasMessages, "messages"), ...when(facts.hasTasks, "tasks"), TAB.pools, TAB.usage, TAB.processes, ...when(facts.hasHistory, "history")];
    // 0.3 (the redesign, §3): everything about a project, its Overview
    // first (the work at a glance), then its sessions, the Apps they ran
    // in (AJ-E3104), its pool's tasks,
    // its pools (AJ-E3104: its own and yours), its sessions' subagents,
    // its usage (there even with none: it says so, AJ-E2805 R2) and its
    // past sessions.
    case "project":
      return [TAB.overview, TAB.sessions, { ...TAB.apps, source: "the apps its sessions ran in" }, { ...TAB.tasks, source: "its pool's tasks, kept by AI Juggler on this computer" }, TAB.pools, TAB.subagents, TAB.usage, { ...TAB.history, source: "the tools' own session files" }];
    // D-62 (AJ-E3204): what's open there and its room, then all its
    // sessions, the projects it worked in, its usage (where its folder
    // is known), its open sessions' processes, and its settings.
    case "app":
      return [TAB.overview, TAB.sessions, TAB.projects, TAB.pools, ...when(facts.cli, "launch"), ...when(facts.hasUsage, "usage"), ...when(facts.live, "processes"), TAB.settings];
    // D-62 (AJ-E3205): its limits and what's open on it, then its Apps,
    // sessions, projects, usage and its own name and terms.
    case "subscription":
      return [TAB.overview, TAB.apps, TAB.sessions, TAB.projects, TAB.pools, TAB.usage, { ...TAB.settings, source: "your choices for this account" }];
    case "conversation":
      return [TAB.overview, ...when(facts.hasSubagents, "subagents"), ...when(facts.hasUsage, "usage"), ...when(facts.live, "processes")];
    // AJ-E3104: its Apps, then everything they hold.
    case "tool":
      return [TAB.overview, { ...TAB.apps, source: "AI Juggler's apps of this tool" }, TAB.sessions, TAB.projects, TAB.pools];
    // AJ-E3104: this Mac's page: its state and what's using it, then
    // everything on it, counted. A paired Mac's (AJ-E705): its peer's
    // answers, its processes and what can be done there.
    case "machine":
      return facts.local
        ? [{ ...TAB.overview, source: `measured on ${THIS_MACHINE_IN_TEXT}` }, TAB.sessions, { ...TAB.apps, source: `the apps and profiles on ${THIS_MACHINE_IN_TEXT}` }, TAB.projects, TAB.pools, TAB.processes]
        : [TAB.overview, TAB.processes, TAB.actions];
    case "remote-session":
      return [TAB.overview, TAB.quality, TAB.processes, TAB.actions];
    case "pool":
      return [TAB.members, TAB.tasks, TAB.messages, TAB.access];
  }
}

const MEMORY = (kind: PageKind) => `aij.page.tab.${kind}`;

/** The tab this type of page was last left on, when it still has it;
 * else its body. Remembered per type, never carried from one type to
 * another (AJ-E1534), nor from one data folder to another (rc32). */
export function rememberedTab(kind: PageKind, tabs: PageTab[]): PageTabId {
  // Storage unavailable: the body.
  const saved = readKept(MEMORY(kind));
  return tabs.find((tab) => tab.id === saved)?.id ?? tabs[0]!.id;
}

/** The tab this type of page was last left on, whether or not this
 * item has it (`null`: none remembered). */
export function rememberedTabId(kind: PageKind): PageTabId | null {
  return readKept(MEMORY(kind)) as PageTabId | null;
}

export function rememberTab(kind: PageKind, tab: PageTabId): void {
  keep(MEMORY(kind), tab);
}

/** The event the open page of a kind switches tabs on. */
export const PAGE_TAB_OF = "aij:page-tab-of";

/** AJ-E3104: open `kind`'s pages on `tab`, the one open now too (a
 * notice about the page already shown still lands on its tab). */
export function openOnTab(kind: PageKind, tab: PageTabId): void {
  rememberTab(kind, tab);
  window.dispatchEvent(new CustomEvent<{ kind: PageKind; tab: PageTabId }>(PAGE_TAB_OF, { detail: { kind, tab } }));
}
