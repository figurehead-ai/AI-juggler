/**
 * 0.3 — what a switch's handover told the other tool, and saving one from
 * a session's page (ENGINEERING-GUIDE U3: one thin file per feature). In
 * the preview, the seams `__AIJ_HANDOVER_SUMMARY__` (by edge id; `null`
 * for a switch that wrote none), `__AIJ_HANDOVER__` (the sheet),
 * `__AIJ_HANDOVER_SAVED__` / `__AIJ_HANDOVER_SAVE_ERROR__` and
 * `__AIJ_HANDOVER_REVEALED__` (what Reveal in Finder was asked for)
 * stand in.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { HandoverSheetInfo } from "../../types/generated/HandoverSheetInfo";
import type { HandoverSummaryInfo } from "../../types/generated/HandoverSummaryInfo";
import type { SavedHandoverInfo } from "../../types/generated/SavedHandoverInfo";

export type { HandoverSheetInfo, HandoverSummaryInfo, SavedHandoverInfo };

const seams = () =>
  window as unknown as {
    __AIJ_HANDOVER_SUMMARY__?: Record<string, HandoverSummaryInfo | null>;
    __AIJ_HANDOVER__?: HandoverSheetInfo;
    __AIJ_HANDOVER_SAVED__?: SavedHandoverInfo;
    __AIJ_HANDOVER_SAVE_ERROR__?: string;
    __AIJ_HANDOVER_REVEALED__?: string[];
  };

const SAMPLE: HandoverSheetInfo = {
  from_tool: "Claude Code",
  from_account: "Work Max",
  to_tool: "Codex",
  to_account: "Personal",
  created_at_ms: 1_792_700_000_000,
  project_root: "~/Dev/payments-api",
  fidelity: "Full story, fresh start",
  carries: ["the conversation (24 turns)", "1 rules file"],
  not_carried: ["12 earlier turns: they didn't fit, so the handover names what was asked in them", "deploy.md: a key or password was found in it, or it didn't fit"],
  withheld: [{ what: "API keys", count: 2 }],
  kept: [],
  brief: "You are continuing a session that previously ran under another account or tool.\n\n> Refunds: the API is done, the webhook is next.\n",
  line: "Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note",
};

const SAMPLE_SAVED: SavedHandoverInfo = {
  path: "/Users/you/Library/Application Support/AI Juggler/exports/handover-preview",
  shown: "~/Library/Application Support/AI Juggler/exports/handover-preview",
  line: "Hidden: 2 API keys",
};

/** A switch's handover in one line; null when it wrote none (a switch on
 * the same tool copies the whole session). */
export async function handoverSummary(edgeId: string): Promise<HandoverSummaryInfo | null> {
  // The preview has none unless a fixture gives one, by edge id or `*`
  // for any (the done screens the gallery already shows stay as they are).
  if (!hasLiveBackend()) {
    const set = seams().__AIJ_HANDOVER_SUMMARY__ ?? {};
    return (edgeId in set ? set[edgeId] : set["*"]) ?? null;
  }
  return invoke<HandoverSummaryInfo | null>("handover_summary", { edgeId });
}

/** A switch's handover, to read (nothing is written). */
export async function handoverOpen(edgeId: string): Promise<HandoverSheetInfo> {
  if (!hasLiveBackend()) return seams().__AIJ_HANDOVER__ ?? SAMPLE;
  return invoke<HandoverSheetInfo>("handover_open", { edgeId });
}

/** Saves a session's handover, keys handled as the user chose. */
export async function handoverSave(sessionId: string): Promise<SavedHandoverInfo> {
  if (!hasLiveBackend()) {
    const failure = seams().__AIJ_HANDOVER_SAVE_ERROR__;
    if (failure) throw failure;
    return seams().__AIJ_HANDOVER_SAVED__ ?? SAMPLE_SAVED;
  }
  return invoke<SavedHandoverInfo>("handover_save", { sessionId });
}

/** Reveal in Finder: a handover AI Juggler saved. */
export async function handoverReveal(path: string): Promise<void> {
  if (!hasLiveBackend()) {
    seams().__AIJ_HANDOVER_REVEALED__ = [...(seams().__AIJ_HANDOVER_REVEALED__ ?? []), path];
    return;
  }
  return invoke<void>("handover_reveal", { path });
}
