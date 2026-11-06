/**
 * Switching (the golden path): the hand-over, the preflight, door count
 * and undo.
 */
import { call, listen, liveBackend as hasBackend } from "./transport";
import type { RedactionInfoData, RedactionItemData } from "./redaction";
import type { DestinationsInfo } from "../../types/generated/DestinationsInfo";
import type { LaunchResult } from "../../types/generated/LaunchResult";
import { previewHooks } from "../preview-hooks";

/** The juggle: move a session's context to another account and resume it. */
export async function sessionHandover(
  sessionId: string,
  targetProfileId: string,
): Promise<LaunchResult> {
  return call("session_handover", { sessionId, targetProfileId });
}

/** AJ-E205: the preflight verdict for one switch (rendered by the
 * freeze dialog before anything executes). */
export interface PreflightInfoData {
  mechanism: string;
  fidelity: string;
  /** AJ-E1543: the label in plain words, what the screen shows. */
  fidelity_words: string;
  score: number;
  warnings: string[];
  zombies: string[];
  diff_snapshotted: boolean;
  source_live: boolean;
  /** 0.4: what moves (its title) and the account it's on: the confirm step names both. */ source_title?: string | null; source_account?: string; source_family?: number; /** ADR-0028: what wakes it by itself (moves to the copy, off on the original), what stays with the account, and whether taking it needs Desktop. */ automation?: string[]; automation_stays?: string[]; automation_needs_desktop?: boolean;
  /** AJ-E1540: live, but AI Juggler can't stop it (started outside it). */
  source_keeps_running: boolean;
  /** AJ-E2108: a session another app runs: only read, left as it is, and
   * continued as a new session here. */
  source_outside?: boolean;
  /** The app it's open in now ("Claude", "Terminal"), when known. */
  source_app?: string | null;
  experimental_copy: string;
  /** AJ-E206 §8 door verdict: "one-click" | "wizard". */
  door: "one-click" | "wizard";
  /** Wizard step titles in §8 order (empty for 1-click). */
  wizard_steps: string[];
  /** Why the wizard opened (one line per failed §8 condition). */
  wizard_reasons: string[];
  /** The target is the project's dominant account. */
  usual_account: boolean;
  /** The carry-list the door renders (AJ-E206). */
  carry: CarryListData;
  /** AJ-E208: the destination Provider's redaction policy. */
  redaction: RedactionInfoData;
  /** AJ-E208: detected secrets (masked) — the per-item un-redact list. */
  redaction_items: RedactionItemData[];
  /** AJ-E808: validated per the fidelity register (absent = not validated). */
  validated?: boolean;
  /** What goes along, in plain words, a line on each. */
  carry_items: CarryItemData[];
  /** The Target step's first line: what moves, said positively. */
  move_line: string;
  /** Whether the keys-and-passwords question applies, and who reads. */
  secrets: SecretsData;
  /** The target's tool id (named from the catalog), and its account (its room: the one runway). */
  target_tool: string;
  target_account?: string;
  /** 0.3.5: where it can continue (Desktop or CLI), and the default. */
  destinations?: DestinationsInfo;
}

/** One thing a switch moves, recreates or leaves behind. */
export interface CarryItemData {
  label: string;
  hint: string;
  how: "moves" | "recreated" | "stays";
  kind?: string; // `mcp`, `skills`, …: joined with the set-up check
}

/** The keys question: whether it applies (the words go to another tool), and who reads ("Codex (OpenAI)"). */
interface SecretsData {
  asks: boolean;
  reader: string;
}

/** The carry-list grouped by disposition (AJ-E206). */
interface CarryListData {
  carried: string[];
  regenerated: string[];
  dropped: string[];
}

/** AJ-E206: the executed switch's handles (the edge is the undo handle). */
export interface SwitchExecutedData {
  session_id: string;
  pid: number;
  edge_id: string;
  source_session_id: string;
  /** 0.3.5: "desktop" | "cli"; for desktop, the conversation now in the
   * Claude app (`conversation:<account>:<id>`), whose page opens after. */
  continued_in?: string;
  conversation_key?: string | null;
  /** ADR-0028: the sessions started from it that moved with it, and where they nest ("now" | "restart" | ""). */ family_moved?: number; family_nests?: string; family_place?: string | null; /** ADR-0028: the automation handed over, the copy's app to restart for it, whether the original's is off only once its app closes, and what stayed. */ automation_moved?: string[]; automation_copy_place?: string | null; automation_original_waits?: boolean; automation_kept?: string[];
}

/** AJ-E206 §5: the switch screen counts the door it shows (local
 * diagnostics); the preflight itself only reads. The preview records it
 * in `__AIJ_DOOR_COUNTS__`. */
export async function switchDoorCount(door: string, steps: string[]): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_DOOR_COUNTS__?: Array<{ door: string; steps: string[] }> };
    (w.__AIJ_DOOR_COUNTS__ ??= []).push({ door, steps });
    return;
  }
  await call("switch_door_count", { door, steps });
}

/** AJ-E205: preflight — ordered checks stop BEFORE any byte moves;
 * warnings render in the freeze dialog. */
export async function switchPreflight(
  sessionId: string,
  targetProfileId: string,
): Promise<PreflightInfoData> {
  if (!hasBackend) {
    const seam = (window as unknown as { __AIJ_SWITCH_PREFLIGHT__?: Partial<PreflightInfoData> })
      .__AIJ_SWITCH_PREFLIGHT__;
    // AJ-E2108: a session another app runs (its key) is only read.
    const outside = sessionId.startsWith("conversation:") ? { source_outside: true } : {};
    const base = { ...previewHooks().switching.DEFAULT_PREFLIGHT, ...outside, destinations: previewHooks().switchDestinations.previewDestinations(sessionId, targetProfileId) };
    // Merge over the default so fixtures that predate a field keep
    // working (the seam overrides only what it defines).
    if (seam) {
      return {
        ...base,
        ...seam,
        redaction: { ...base.redaction, ...seam.redaction },
        secrets: { ...base.secrets, ...seam.secrets },
      } as PreflightInfoData;
    }
    return base;
  }
  return call("switch_preflight", { sessionId, targetProfileId });
}

/** AJ-E206 §8.2: undoes a completed switch — stops the continuation
 * instance, resumes the source's native session, records the
 * compensating edge. User-click only. */
export async function switchUndo(edgeId: string): Promise<LaunchResult> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_SWITCH_UNDO_CALLS__?: string[];
      __AIJ_SWITCH_UNDO_FAIL__?: string;
    };
    // The guardrail seam: undo is observable so tests can prove it
    // fires ONLY from the click.
    seams.__AIJ_SWITCH_UNDO_CALLS__ = [
      ...(seams.__AIJ_SWITCH_UNDO_CALLS__ ?? []),
      edgeId,
    ];
    if (seams.__AIJ_SWITCH_UNDO_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_SWITCH_UNDO_FAIL__);
    }
    return { session_id: `sess-undo-${Date.now()}`, pid: 0 };
  }
  return call("switch_undo", { edgeId });
}

/** 0.3.9: the switch pipeline's stage as it moves (`switch://progress`:
 * copying, checking, starting, confirming). */
export function onSwitchProgress(handler: (stage: string) => void): Promise<() => void> {
  return listen<string>("switch://progress", (event) => handler(event.payload));
}
