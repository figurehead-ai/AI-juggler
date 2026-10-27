/**
 * AJ-E1533 — one page model (ADR-0019 §3: the shell renders screens
 * from a registry). Every item opens the same kind of page: an identity
 * header (its type, mark, name, parent and a few facts), then its tabs,
 * the first being the item's body — a session's terminal, a project's
 * sessions. A tab shows only when it has something to show; the actions
 * live in the header and its ⋯ menu, never in a tab of their own.
 */

export type PageKind = "session" | "conversation" | "project" | "account" | "tool" | "machine" | "remote-session" | "pool";

/** The type, in words, on every page's header. A conversation started
 * outside AI Juggler is a session too (it says "outside AI Juggler"). */
export const PAGE_LABELS: Record<PageKind, string> = {
  session: "Session",
  conversation: "Session",
  project: "Project",
  account: "Account",
  tool: "Tool",
  machine: "Machine",
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
  | "access";

export type PageTab = {
  id: PageTabId;
  label: string;
  /** Where its numbers come from, in plain words (the footer). */
  source: string;
};

const TAB: Record<PageTabId, PageTab> = {
  terminal: { id: "terminal", label: "Terminal", source: "this Mac's process monitor" },
  window: { id: "window", label: "Window preview", source: "this Mac's process monitor" },
  sessions: { id: "sessions", label: "Sessions", source: "the tools' own session files" },
  overview: { id: "overview", label: "Overview", source: "this Mac's process monitor" },
  usage: { id: "usage", label: "Usage", source: "usage tracking" },
  quality: { id: "quality", label: "Quality", source: "this Mac's process monitor" },
  processes: { id: "processes", label: "Processes", source: "this Mac's process monitor" },
  history: { id: "history", label: "History", source: "the session's transcript" },
  actions: { id: "actions", label: "Actions", source: "the paired Mac" },
  members: { id: "members", label: "Members", source: "AI Juggler's pools and the sessions' own transcripts" },
  messages: { id: "messages", label: "Messages", source: "AI Juggler's own messages, on this Mac" },
  tasks: { id: "tasks", label: "Tasks", source: "the pool's tasks, kept by AI Juggler on this Mac" },
  subagents: { id: "subagents", label: "Subagents", source: "the tool's own records of its subagents, read-only" },
  access: { id: "access", label: "Agent access", source: "your choices for this pool" },
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
  /** AJ-E1520: a session whose tool records subagents, and it started
   * some. */
  hasSubagents?: boolean;
};

/** Where an Overview's numbers come from, by page (P21: never one
 * source named for all). */
const OVERVIEW_SOURCE: Partial<Record<PageKind, string>> = {
  project: "AI Juggler's projects and the tools' session files",
  account: "the account's settings and usage tracking",
  tool: "the tool's install and its accounts",
  // AJ-E2108: where it runs, and its last messages from its transcript's end.
  session: "the apps' own session files and the end of its transcript",
  conversation: "the apps' own session files and the end of its transcript",
};

/** The page's tabs, its body first. */
export function pageTabs(kind: PageKind, facts: TabFacts = {}): PageTab[] {
  const source = OVERVIEW_SOURCE[kind];
  const tabs = tabsOf(kind, facts);
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
        ? [TAB.window, TAB.overview, ...when(facts.hasSubagents, "subagents"), ...when(facts.hasMessages, "messages"), ...when(facts.hasTasks, "tasks"), TAB.usage, TAB.processes, ...when(facts.hasHistory, "history")]
        : [{ ...TAB.terminal, label: "Overview" }, ...when(facts.hasSubagents, "subagents"), ...when(facts.hasMessages, "messages"), ...when(facts.hasTasks, "tasks"), TAB.usage, TAB.processes, ...when(facts.hasHistory, "history")];
    case "project":
      return [TAB.sessions, TAB.overview, ...when(facts.hasUsage, "usage")];
    case "account":
      return [TAB.overview, ...when(facts.hasSessions, "sessions")];
    case "conversation":
      return [TAB.overview, ...when(facts.hasSubagents, "subagents"), ...when(facts.hasUsage, "usage"), ...when(facts.live, "processes")];
    case "tool":
      return [TAB.overview];
    case "machine":
      return [TAB.overview, TAB.processes, TAB.actions];
    case "remote-session":
      return [TAB.overview, TAB.quality, TAB.processes, TAB.actions];
    case "pool":
      return [TAB.members, TAB.tasks, TAB.messages, TAB.access];
  }
}

const MEMORY = (kind: PageKind) => `aij.page.tab.${kind}`;

/** The tab this type of page was last left on, when it still has it;
 * else its body. Remembered per type, never carried from one type to
 * another (AJ-E1534). */
export function rememberedTab(kind: PageKind, tabs: PageTab[]): PageTabId {
  let saved: string | null = null;
  try {
    saved = localStorage.getItem(MEMORY(kind));
  } catch {
    // Storage unavailable: the body.
  }
  return tabs.find((tab) => tab.id === saved)?.id ?? tabs[0]!.id;
}

export function rememberTab(kind: PageKind, tab: PageTabId): void {
  try {
    localStorage.setItem(MEMORY(kind), tab);
  } catch {
    // Storage unavailable: this visit only.
  }
}
