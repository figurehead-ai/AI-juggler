/**
 * AJ-E2104 — a desktop with several places open, for the preview and the
 * rail's and Home's specs: the Claude app's Work and Personal profiles,
 * Terminal, ChatGPT (Codex); sessions asking, working, idle and done; one
 * ended, one inactive, one background run. `extra` adds that many more
 * open sessions spread over the places (the 40-session rail).
 *
 * Preview data only: the app reads `conversations_list`. The preview
 * shows it when `sessionStorage["aij.preview.places"]` is "1".
 */
import type { ConversationInfo } from "../types/generated/ConversationInfo";
import type { PlaceInfo } from "../types/generated/PlaceInfo";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";

const CLAUDE_WORK: PlaceInfo = { app: "Claude", shown: "Claude app", app_id: "claude-app", profile: "Work Max", instance: "Work", pid: 4242, kind: "desktop", app_pid: 4100, label: "Claude app · Work" };
const CLAUDE_PERSONAL: PlaceInfo = { app: "Claude", shown: "Claude app", app_id: "claude-app", profile: "Personal", instance: "Personal", pid: 4343, kind: "desktop", app_pid: 4101, label: "Claude app · Personal" };
const TERMINAL: PlaceInfo = { app: "Terminal", shown: "Terminal", app_id: "terminal", profile: "Work Max", instance: null, pid: 5150, kind: "terminal", app_pid: 5100, label: "Terminal" };
const CODEX: PlaceInfo = { app: "ChatGPT", shown: "ChatGPT", app_id: "codex-app", profile: "Codex", instance: null, pid: 6160, kind: "desktop", app_pid: 6100, label: "ChatGPT" };
export const PREVIEW_PLACES = [CLAUDE_WORK, CLAUDE_PERSONAL, TERMINAL, CODEX];

const TITLES = ["Speed up the search index", "Write the onboarding copy", "Fix the date picker on Safari", "Move the jobs to the new queue", "Add retries to the uploader", "Review the API error messages", "Port the settings to the new form", "Split the billing module"];

export function placesConversations(now: number, extra = 0): ConversationInfo[] {
  let pid = 7000;
  const open = (id: string, place: PlaceInfo, rest: Partial<ConversationInfo>): ConversationInfo => {
    pid += 1;
    return {
      id,
      tool: place.app_id === "codex-app" ? "codex" : "claude-code",
      account_id: place.profile === "Personal" ? "claude-personal" : place.app_id === "codex-app" ? "codex" : "claude-work",
      account_label: place.profile ?? "Work Max",
      project_id: "proj-ai-juggler",
      cwd: "/Users/you/Dev/ai-juggler",
      title: id,
      git_branch: "main",
      started_ms: now - 3_600_000,
      last_activity_ms: now - 60_000,
      tokens: null,
      live: true,
      session_id: null,
      places: [{ ...place, pid }],
      pid,
      state: "idle",
      probably: false,
      viewed_ms: null,
      background: false,
      inactive: false,
      subtask_count: null,
      ...rest,
    };
  };
  const rows: ConversationInfo[] = [
    open("outside-1", CLAUDE_WORK, { title: "Refactor the settings screen", state: "working", viewed_ms: now - 120_000 }),
    open("outside-3", CLAUDE_WORK, { title: "Migrate the billing webhooks", project_id: "proj-payments", cwd: "/Users/you/Dev/payments-api", state: "asks_you", last_activity_ms: now - 4 * 60_000 }),
    open("outside-4", CLAUDE_PERSONAL, { title: "Plan the trip notes app", project_id: null, cwd: "/Users/you/Dev/trip-notes", viewed_ms: now - 40 * 60_000, last_activity_ms: now - 45 * 60_000 }),
    open("outside-5", CLAUDE_PERSONAL, { title: "Tidy the blog's images", project_id: null, cwd: "/Users/you/Dev/blog", state: "done_review", last_activity_ms: now - 12 * 60_000 }),
    open("outside-6", TERMINAL, { title: "Fix the flaky login test", state: "working", git_branch: "fix/login-test", last_activity_ms: now - 20_000 }),
    open("outside-7", TERMINAL, { title: "Nightly import run", state: "working", background: true }),
    open("outside-9", TERMINAL, { title: "Answer the release question", state: "asks_you", probably: true, last_activity_ms: now - 2 * 60_000 }),
    open("outside-10", CODEX, { title: "Draft the migration plan", project_id: "proj-payments", cwd: "/Users/you/Dev/payments-api", state: null, last_activity_ms: now - 15 * 60_000 }),
    open("outside-2", CLAUDE_PERSONAL, { title: "Add pagination to the orders endpoint", project_id: "proj-payments", cwd: "/Users/you/Dev/payments-api", live: false, places: [], pid: null, state: null, last_activity_ms: now - 5 * 3_600_000 }),
    open("outside-8", CLAUDE_WORK, { title: "Write the release notes", live: false, places: [], pid: null, state: null, inactive: true, last_activity_ms: now - 26 * 3_600_000 }),
  ];
  for (let n = 0; n < extra; n += 1) {
    const place = PREVIEW_PLACES[n % PREVIEW_PLACES.length]!;
    rows.push(open(`more-${n}`, place, { title: TITLES[n % TITLES.length]!, state: n % 5 === 0 ? "working" : "idle", last_activity_ms: now - (n + 3) * 7 * 60_000 }));
  }
  return rows;
}

/** 0.3.3: the apps and profiles remembered on that desktop: both Claude
 * profiles and ChatGPT running (Work kept alive), a Lab profile kept alive
 * that's down, and an old one; the ended "outside-2" was last open in
 * Claude · Personal. */
export function knownPlacesPreview(now: number): { places: KnownPlaceInfo[]; sessions: Record<string, string> } {
  const row = (over: Partial<KnownPlaceInfo> & Pick<KnownPlaceInfo, "id" | "app_id" | "app" | "label">): KnownPlaceInfo => ({
    instance: null,
    profile: null,
    app_pid: null,
    keep_alive: false,
    paused: null,
    account_id: null,
    seen_ms: now - 3_600_000,
    cannot_start: null,
    recent: [],
    ...over,
  });
  return {
    places: [
      row({ id: "claude-app|/Users/you/Profiles/Work", app_id: "claude-app", app: "Claude", instance: "Work", profile: "Work Max", label: "Claude · Work Max", app_pid: CLAUDE_WORK.app_pid, keep_alive: true, account_id: "claude-work", recent: ["claude-code/outside-3", "claude-code/outside-1", "claude-code/outside-8"] }),
      row({ id: "claude-app|/Users/you/Profiles/Personal", app_id: "claude-app", app: "Claude", instance: "Personal", profile: "Personal", label: "Claude · Personal", app_pid: CLAUDE_PERSONAL.app_pid, account_id: "claude-personal", recent: ["claude-code/outside-5", "claude-code/outside-4", "claude-code/outside-2"] }),
      row({ id: "codex-app|", app_id: "codex-app", app: "ChatGPT", label: "ChatGPT", app_pid: CODEX.app_pid, account_id: "codex" }),
      row({ id: "claude-app|/Users/you/Profiles/Lab", app_id: "claude-app", app: "Claude", instance: "Lab", profile: "Lab", label: "Claude · Lab", keep_alive: true, seen_ms: now - 20 * 60_000 }),
      row({ id: "claude-app|/Users/you/Profiles/Old", app_id: "claude-app", app: "Claude", instance: "Old", profile: "Old", label: "Claude · Old", seen_ms: now - 9 * 24 * 3_600_000 }),
    ],
    sessions: { "claude-code/outside-2": "claude-app|/Users/you/Profiles/Personal" },
  };
}
