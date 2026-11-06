/**
 * Where a switch can go: targets, the work graph, epoch resume and the
 * pack export.
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import { MOCK_ACCOUNTS, MOCK_SESSIONS } from "../../preview/fixtures";
import type { HandoverSummaryInfo } from "../../types/generated/HandoverSummaryInfo";
import type { LaunchResult } from "../../types/generated/LaunchResult";
import type { SavedHandoverInfo } from "../../types/generated/SavedHandoverInfo";

/** AJ-E209: one ranked, badged switch destination (the handover
 * section's rows — §Q6 extended with switch cost + return bonus).
 * Suggest-only: the click is the user's. */
export interface SwitchTargetData {
  recipe_id: string;
  /** The target's Provider id (AJ-E1103 counts suggestions by it). */
  provider?: string;
  account_id: string;
  account_label: string;
  /** Ordered-pair badge wire name ("Full story, fresh start · Beta" — never "lossless"
   * below F4; cross-Provider always carries Beta, D-04/D-11). */
  badge: string;
  /** Validated state (false until E210's fixtures flip cells). */
  validated: boolean;
  cross_provider: boolean;
  /** The pair's Drop list (the badge's honest companion). */
  drops: string[];
  score: number | null;
  filtered_reason: string | null;
  /** 0.3.4: why it's held back, when a step of the user's fixes it:
   * "set_up", its tool's first-run setup isn't done there (a session
   * started on it once does it). */
  held_back?: "set_up" | null;
  /** 0.3.5: its tool's own first run isn't done there; the switch does it. */
  first_run?: boolean;
  top_factors: string[];
  factors: [string, number][];
  confidence: string;
  tte_min: number | null;
  suggested: boolean;
}

/** The preview's two tools, named once: Claude Code as its first
 * account's, Codex for the rows that move to it (ADR-0019 keeps tool ids
 * out of the code). */
const PREVIEW_CLAUDE = MOCK_ACCOUNTS[0]!.provider;
const PREVIEW_CODEX = "codex";

/** The no-backend switch-targets default (preview mode shows a
 * realistic ranked, badged set; fixtures override wholesale). */
const DEFAULT_SWITCH_TARGETS: SwitchTargetData[] = [
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
function previewSwitchTargets(sessionId: string): SwitchTargetData[] {
  const own = MOCK_SESSIONS.find((session) => session.id === sessionId)?.account_label;
  const rows = DEFAULT_SWITCH_TARGETS.filter((row) => row.account_label !== own);
  const best = rows.filter((row) => row.filtered_reason === null).sort((a, b) => (b.score ?? 0) - (a.score ?? 0))[0];
  return rows.map((row) => ({ ...row, suggested: row === best }));
}

/** AJ-E209 Req.1/3: the ranked + badged destinations for one source
 * session. Read-only — ranking suggests, the user clicks. */
export async function switchTargets(sessionId: string): Promise<SwitchTargetData[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_SWITCH_TARGETS__?: SwitchTargetData[] }).__AIJ_SWITCH_TARGETS__ ?? previewSwitchTargets(sessionId);
  }
  return invoke<SwitchTargetData[]>("switch_targets", { sessionId });
}

/** AJ-E209: one epoch node on a Work's timeline. */
export interface EpochNodeData {
  id: string;
  provider: string;
  account_label: string;
  kind: string;
  /** A native session exists AND its transcript survives (the
   * Resume action's honest gate). */
  resumable: boolean;
  session_id?: string | null;
}

/** AJ-E209: one labelled timeline edge. */
export interface GraphEdgeData {
  /** The edge's id (what opens its handover). */
  id?: string;
  from: string;
  to?: string | null;
  /** What carried, in plain words ("Full story, fresh start"; "Switched back · …"). */
  label: string;
  failed_reason?: string | null;
  switch_back: boolean;
  /** 0.3: its handover in one line, when the switch wrote one (to another
   * tool); none for one that copied the session. */
  handover?: HandoverSummaryInfo | null;
}

/** AJ-E209: one Work item's timeline (epochs past the collapse cut). */
export interface WorkGraphData {
  title: string;
  epochs: EpochNodeData[];
  edges: GraphEdgeData[];
  /** Older epochs collapsed away ("+n switches" — never unbounded). */
  collapsed: number;
  /** The S-F9 hop line (inform, never block). */
  hop_note?: string | null;
}

/** The no-backend graph default (preview mode; fixtures override). */
const DEFAULT_SWITCH_GRAPH: WorkGraphData[] = [
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

/** AJ-E209 Req.4: the per-Work Switch Graph timelines for one
 * project. Read-only. */
export async function switchGraphOverview(projectId: string): Promise<WorkGraphData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SWITCH_GRAPH__?: WorkGraphData[] })
        .__AIJ_SWITCH_GRAPH__ ?? DEFAULT_SWITCH_GRAPH
    );
  }
  return invoke<WorkGraphData[]>("switch_graph_overview", { projectId });
}

/** AJ-E209 Req.5: resumes an epoch's native session (the graph's
 * Resume action — the user's click, never automatic). */
export async function epochResume(epochId: string): Promise<LaunchResult> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_EPOCH_RESUME_CALLS__?: string[];
      __AIJ_EPOCH_RESUME_FAIL__?: string;
    };
    // The guardrail seam: resume is observable so tests can prove it
    // fires ONLY from the click.
    seams.__AIJ_EPOCH_RESUME_CALLS__ = [...(seams.__AIJ_EPOCH_RESUME_CALLS__ ?? []), epochId];
    if (seams.__AIJ_EPOCH_RESUME_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_EPOCH_RESUME_FAIL__);
    }
    return { session_id: `sess-resume-${Date.now()}`, pid: 0 };
  }
  return invoke<LaunchResult>("epoch_resume", { epochId });
}

/** AJ-E209 Req.5: exports an epoch's pack through the AJ-E208 GLOBAL
 * redaction policy — returns where it went and what was hidden. */
export async function packExport(epochId: string): Promise<SavedHandoverInfo> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_PACK_EXPORT_CALLS__?: string[];
      __AIJ_PACK_EXPORT_FAIL__?: string;
    };
    // The guardrail seam: exports are observable so tests can prove
    // they fire ONLY from the click.
    seams.__AIJ_PACK_EXPORT_CALLS__ = [...(seams.__AIJ_PACK_EXPORT_CALLS__ ?? []), epochId];
    if (seams.__AIJ_PACK_EXPORT_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_PACK_EXPORT_FAIL__);
    }
    const path = `~/Library/Application Support/AI Juggler/exports/pack-preview-${Date.now()}`;
    return { path, shown: path, line: null };
  }
  return invoke<SavedHandoverInfo>("pack_export", { epochId });
}
