/**
 * AJ-E1511 — opening a saved context pack (ENGINEERING-GUIDE U3: one thin
 * file per feature): read and check it, then start a session primed with
 * it. In the preview, the seams `__AIJ_PACK_DIR__` (the picked folder),
 * `__AIJ_PACK__` (what it holds, or `__AIJ_PACK_ERROR__`) and
 * `__AIJ_PACK_STARTED__` (what was started) stand in.
 */
import { invoke, openDialog } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { PackOpenInfo } from "../../types/generated/PackOpenInfo";
import type { PackStartedInfo } from "../../types/generated/PackStartedInfo";

export type { PackOpenInfo, PackStartedInfo };

const seams = () =>
  window as unknown as {
    __AIJ_PACK_DIR__?: string | null;
    __AIJ_PACK__?: PackOpenInfo;
    __AIJ_PACK_ERROR__?: string;
    __AIJ_PACK_STARTED__?: { path: string; digest: string; profileId: string; folder: string };
  };

const SAMPLE: PackOpenInfo = {
  digest: "preview",
  path: "~/Library/Application Support/AI Juggler/exports/pack-1",
  from_tool: "Claude Code",
  from_account: "Work Max",
  created_at_ms: 1_792_700_000_000,
  project_root: "~/Dev/payments-api",
  project_here: true,
  project_folder: "/Users/you/Dev/payments-api",
  branch: "feat/refunds",
  turns: 24,
  fidelity: "Full story, fresh start · Beta",
  carries: ["the conversation (24 turns)", "1 rules file", "the uncommitted changes"],
  not_carried: ["Subagents' work: the other tool can't run Claude's subagents"],
  withheld: [{ what: "API keys", count: 2 }],
  brief: "# Handover\n\nRefunds: the API is done, the webhook is next.\n",
};

/** Which pack to open (the folder picker); null when cancelled. */
export async function pickPackFolder(): Promise<string | null> {
  if (!hasLiveBackend()) return seams().__AIJ_PACK_DIR__ ?? null;
  const chosen = await openDialog({ directory: true, multiple: false, title: "Open a handover" });
  return typeof chosen === "string" ? chosen : null;
}

/** Reads and checks the pack; what it holds (nothing is written). */
export async function packsOpenPreview(path: string): Promise<PackOpenInfo> {
  if (!hasLiveBackend()) {
    const failure = seams().__AIJ_PACK_ERROR__;
    if (failure) throw failure;
    return seams().__AIJ_PACK__ ?? SAMPLE;
  }
  return invoke<PackOpenInfo>("packs_open_preview", { path });
}

/** Starts a session on `profileId` in `folder`, primed with the pack. */
export async function packsOpenStart(path: string, digest: string, profileId: string, folder: string): Promise<PackStartedInfo> {
  if (!hasLiveBackend()) {
    seams().__AIJ_PACK_STARTED__ = { path, digest, profileId, folder };
    return { session_id: "sess-live-1", edge_id: "edge-preview" };
  }
  return invoke<PackStartedInfo>("packs_open_start", { path, digest, profileId, folder });
}
