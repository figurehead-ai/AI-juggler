/** The design preview's switching data: the preflight, targets, work graph and validation matrix (moved from lib/ipc, AJ-E2018). */
import type { PreflightInfoData } from "../lib/ipc/switch-preflight";
import type { SwitchTargetData, WorkGraphData } from "../lib/ipc/switch-targets";
import type { ValidationCellData } from "../lib/ipc/validation";
import { MOCK_ACCOUNTS, MOCK_SESSIONS } from "./fixtures";

/** The no-backend preflight default (fixtures merge over it —
 * AJ-E206 door facts + AJ-E208 redaction policy). */
export const DEFAULT_PREFLIGHT: PreflightInfoData = {
  mechanism: "transplant",
  fidelity: "F3",
  fidelity_words: "Same conversation, another app",
  score: 0.85,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: true,
  source_title: "Fix the checkout flow", source_account: "Work",
  source_keeps_running: false,
  experimental_copy:
    "Beta, not proven yet: the whole session is carried over, with its subagents and file history. The source stays resumable.",
  door: "wizard",
  wizard_steps: ["Target", "What's carried", "Confirm"],
  // D-03 amendment: another account is no reason; an unproven pair is.
  wizard_reasons: ["This switch hasn't been proven on a real setup yet."],
  usual_account: false,
  // A same-tool switch, as the fidelity table has it (AJ-E1540/E1541):
  // the whole session moves, and the task list is rebuilt from it.
  carry: {
    carried: ["The whole conversation", "Project rules", "Your uncommitted changes", "Subagents' work"],
    regenerated: ["The task list"],
    dropped: [],
  },
  redaction: {
    global: "strip",
    override: null,
    effective: "strip",
    source: "global",
    line: "Keys and passwords for Claude Code: hidden (your global choice)",
  },
  redaction_items: [],
  carry_items: [
    { label: "The whole conversation", hint: "Every message from the start, copied as the tool keeps it; the new account picks it up where it stopped.", how: "moves", kind: "chat" },
    { label: "Project rules", hint: "The rule files in the project folder (such as CLAUDE.md or AGENTS.md); they stay there and are read again.", how: "moves", kind: "rules" },
    { label: "Your uncommitted changes", hint: "Changes you haven't committed stay in the folder; the new session works in the same folder.", how: "moves", kind: "changes" },
    { label: "Subagents' work", hint: "Work done by helper agents inside the session.", how: "moves", kind: "subagents" },
    { label: "The task list", hint: "What the session planned to do next, from its latest version.", how: "recreated", kind: "tasks" },
  ],
  move_line: "The whole conversation is carried over — all 42 messages, its subagents and file history.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: MOCK_ACCOUNTS[0]!.provider, // the mock accounts' tool (no tool id written here: ADR-0019)
};

/** The preview's two tools, named once: Claude Code as its first
 * account's, Codex for the rows that move to it (ADR-0019 keeps tool ids
 * out of the code). */
export const PREVIEW_CLAUDE = MOCK_ACCOUNTS[0]!.provider;

export const PREVIEW_CODEX = "codex";

/** The no-backend switch-targets default (preview mode shows a
 * realistic ranked, badged set; fixtures override wholesale). */
export const DEFAULT_SWITCH_TARGETS: SwitchTargetData[] = [
  // Each row's words match its tool (the 0.2 re-render): another Claude
  // Code account keeps the same conversation; Codex starts fresh from the
  // story, as the backend's pair matrix says. The Claude Code rows carry
  // the preview accounts' own ids, so their room is read as Home reads it
  // (the limit dialog's pick said nothing of Work Max's 58% left).
  {
    recipe_id: "recipe-work-max",
    provider: PREVIEW_CLAUDE,
    account_id: "claude-work",
    account_label: "Work Max",
    badge: "Same conversation, another app · Beta",
    validated: false,
    cross_provider: false,
    drops: [],
    score: 0.62,
    top_factors: ["80% headroom", "used most here"],
    factors: [["80% headroom", 0.36], ["used most here", 0.2], ["quiet", 0.1]],
    confidence: "Computed",
    tte_min: 262,
    filtered_reason: null,
    suggested: true,
  },
  {
    recipe_id: "recipe-personal",
    provider: PREVIEW_CLAUDE,
    account_id: "claude-personal",
    account_label: "Personal",
    badge: "Same conversation, another app · Beta",
    validated: false,
    cross_provider: false,
    drops: [],
    score: 0.41,
    top_factors: ["35% headroom", "quiet"],
    factors: [["35% headroom", 0.16], ["quiet", 0.1]],
    confidence: "Estimate",
    tte_min: 95,
    filtered_reason: null,
    suggested: false,
  },
  {
    recipe_id: "recipe-codex",
    provider: PREVIEW_CODEX,
    account_id: "acct-codex",
    account_label: "Codex",
    badge: "Full story, fresh start · Beta",
    validated: false,
    cross_provider: true,
    drops: ["Subagents' work", "The task list"],
    score: 0.48,
    top_factors: ["53% headroom", "another tool's limits"],
    factors: [["53% headroom", 0.24], ["another tool's limits", 0.12], ["starts fresh from the story", 0.04]],
    confidence: "Computed",
    tte_min: 180,
    filtered_reason: null,
    suggested: false,
  },
  {
    recipe_id: "recipe-lab",
    provider: PREVIEW_CLAUDE,
    account_id: "acct-lab",
    account_label: "Lab",
    badge: "Same conversation, another app · Beta",
    validated: false,
    cross_provider: false,
    drops: [],
    filtered_reason: "exhausted — resets in 3h",
    top_factors: [],
    factors: [],
    confidence: "Measured",
    score: null,
    tte_min: null,
    suggested: false,
  },
];

/** The preview's targets for one session: never its own account (the
 * backend never offers where it already is), the best open row the
 * suggestion. */
export function previewSwitchTargets(sessionId: string): SwitchTargetData[] {
  const own = MOCK_SESSIONS.find((session) => session.id === sessionId)?.account_label;
  const rows = DEFAULT_SWITCH_TARGETS.filter((row) => row.account_label !== own);
  const best = rows.filter((row) => row.filtered_reason === null).sort((a, b) => (b.score ?? 0) - (a.score ?? 0))[0];
  return rows.map((row) => ({ ...row, suggested: row === best }));
}

/** The no-backend graph default (preview mode; fixtures override). */
export const DEFAULT_SWITCH_GRAPH: WorkGraphData[] = [
  {
    title: "Login refactor",
    epochs: [
      { id: "ep-1", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-a" },
      { id: "ep-2", provider: PREVIEW_CODEX, account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-b" },
      { id: "ep-3", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-c" },
    ],
    edges: [
      { from: "ep-1", to: "ep-2", label: "Full story, fresh start", switch_back: false },
      { from: "ep-2", to: "ep-3", label: "Switched back · Full story, fresh start", switch_back: true },
    ],
    collapsed: 2,
    hop_note: "3 switches so far ≈41k tokens",
  },
];

/** The no-backend matrix default (mirrors the researched table:
 * one pair with real-tool tests, every pair honestly in beta). */
export const DEFAULT_VALIDATION_MATRIX: ValidationCellData[] = [
  ["claude-code", "claude-code", "transplant", "F3", true],
  ["codex", "codex", "transplant", "F3", false],
  ["zcode", "zcode", "transplant", "F1", false],
  ["claude-code", "codex", "replay", "F1", false],
  ["codex", "claude-code", "replay", "F1", false],
  ["claude-code", "zcode", "replay", "F0", false],
  ["zcode", "claude-code", "replay", "F1", false],
  ["codex", "zcode", "replay", "F0", false],
  ["zcode", "codex", "replay", "F1", false],
  ["claude-code", "gemini-cli", "pack-only", "F0", false],
  ["claude-code", "opencode", "pack-only", "F0", false],
].map(([from, to, mechanism, ceiling, ciGreen]) => ({
  from: from as string,
  to: to as string,
  mechanism: mechanism as string,
  researched_ceiling: ceiling as string,
  advertised: ceiling as string,
  badge: `${ceiling === "F3" ? "Same conversation, another app" : ceiling === "F1" ? "Full story, fresh start" : "Handover notes"} · Beta`,
  validated: false,
  mechanism_green: true,
  ci_green: ciGreen as boolean,
  dogfood: null,
  status: ciGreen
    ? "Automatic tests pass with the real tools; not tried by hand yet"
    : "Automatic tests with the real tools aren't in place; not tried by hand yet",
  notes: [],
}));
