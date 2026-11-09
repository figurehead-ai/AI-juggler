/**
 * Where a switch can go: targets, the work graph, epoch resume and the
 * pack export.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { HandoverSummaryInfo } from "../../types/generated/HandoverSummaryInfo";
import type { LaunchResult } from "../../types/generated/LaunchResult";
import type { SavedHandoverInfo } from "../../types/generated/SavedHandoverInfo";
import { previewHooks } from "../preview-hooks";

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
  /** 0.4.1: its tool isn't signed in there (in a terminal): its row says
   * so instead of its account's room. */
  signed_out?: boolean;
  top_factors: string[];
  factors: [string, number][];
  confidence: string;
  tte_min: number | null;
  suggested: boolean;
}

/** AJ-E209 Req.1/3: the ranked + badged destinations for one source
 * session. Read-only — ranking suggests, the user clicks. */
export async function switchTargets(sessionId: string): Promise<SwitchTargetData[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_SWITCH_TARGETS__?: SwitchTargetData[] }).__AIJ_SWITCH_TARGETS__ ?? previewHooks().switching.previewSwitchTargets(sessionId);
  }
  return call("switch_targets", { sessionId });
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

/** AJ-E209 Req.4: the per-Work Switch Graph timelines for one
 * project. Read-only. */
export async function switchGraphOverview(projectId: string): Promise<WorkGraphData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SWITCH_GRAPH__?: WorkGraphData[] })
        .__AIJ_SWITCH_GRAPH__ ?? previewHooks().switching.DEFAULT_SWITCH_GRAPH
    );
  }
  return call("switch_graph_overview", { projectId });
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
  return call("epoch_resume", { epochId });
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
  return call("pack_export", { epochId });
}
