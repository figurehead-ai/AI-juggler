/**
 * Launch recipes and starting a session: the identity check, the
 * isolation offer and worktrees, and the launch itself.
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import type { GitStatusInfo } from "../../types/generated/GitStatusInfo";
import type { IdentityCheckInfo } from "../../types/generated/IdentityCheckInfo";
import type { IsolationOfferInfo } from "../../types/generated/IsolationOfferInfo";
import type { LaunchResult } from "../../types/generated/LaunchResult";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";
import type { WorktreeRemovalInfo } from "../../types/generated/WorktreeRemovalInfo";
import { previewHooks } from "../preview-hooks";

export async function launchRecipesList(): Promise<LaunchRecipeInfo[]> {
  if (!hasBackend) {
    // AJ-E305 test seam: visual/flow specs inject the recipe set.
    return (
      (window as unknown as { __AIJ_RECIPES__?: LaunchRecipeInfo[] }).__AIJ_RECIPES__ ??
      previewHooks().fixtures.MOCK_PROFILES
    );
  }
  return invoke<LaunchRecipeInfo[]>("launch_recipes_list");
}

/** @deprecated legacy alias (AJ-E107) — use launchRecipesList. */
export const profilesList = launchRecipesList;

export async function createProfile(
  accountId: string,
  name: string,
  model: string | null,
  defaultCwd: string | null,
): Promise<LaunchRecipeInfo> {
  return invoke<LaunchRecipeInfo>("profile_create", {
    accountId,
    name,
    model,
    defaultCwd,
  });
}

/** AJ-E1001: who a Launch setting's Account is signed in as — checked
 * before anything launches (read-only). */
export async function identityCheck(recipeId: string): Promise<IdentityCheckInfo> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_IDENTITY_CHECK__?: Record<string, IdentityCheckInfo> };
    const seeded = seams.__AIJ_IDENTITY_CHECK__?.[recipeId];
    if (seeded) return seeded;
    return {
      state: "verified",
      provider: previewHooks().fixtures.PREVIEW_TOOL.id,
      account_id: "",
      account_label: "",
      model: null,
      expected: null,
      detected: null,
      reason: null,
      alternatives: [],
    };
  }
  return invoke<IdentityCheckInfo>("identity_check", { recipeId });
}

/** What the user chose in the identity-mismatch dialog. */
export type IdentityDecision = { kind: "switch"; recipeId: string } | { kind: "recheck" } | { kind: "cancel" };

/** The event the app's identity dialog listens for (one dialog serves
 * every launch entry point). */
export const IDENTITY_MISMATCH_EVENT = "aij:identity-mismatch";

export type IdentityMismatchRequest = {
  check: IdentityCheckInfo;
  resolve: (decision: IdentityDecision) => void;
};

/** Asks the user (through the app's dialog) what to do about a mismatch. */
function askIdentityDecision(check: IdentityCheckInfo): Promise<IdentityDecision> {
  return new Promise((resolve) => {
    const detail: IdentityMismatchRequest = { check, resolve };
    const delivered = window.dispatchEvent(new CustomEvent(IDENTITY_MISMATCH_EVENT, { detail, cancelable: true }));
    // No dialog mounted (a test, a detached view): never continue silently.
    if (delivered) {
      window.setTimeout(() => {
        if (!(detail as { claimed?: boolean }).claimed) resolve({ kind: "cancel" });
      }, 0);
    }
  });
}

/** AJ-E1002: the worktree offer for launching into `cwd` (`null` when
 * nobody else works there). */
export async function isolationOffer(cwd: string): Promise<IsolationOfferInfo | null> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_ISOLATION_OFFER__?: IsolationOfferInfo | null };
    return seams.__AIJ_ISOLATION_OFFER__ ?? null;
  }
  return invoke<IsolationOfferInfo | null>("isolation_offer", { cwd });
}

/** AJ-E1002: creates the worktree the user confirmed; returns its path. */
export async function worktreeCreate(cwd: string, path: string, branch: string): Promise<string> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_LAUNCH_CALLS__?: string[] };
    seams.__AIJ_LAUNCH_CALLS__ = [...(seams.__AIJ_LAUNCH_CALLS__ ?? []), `worktree:${branch}:${path}`];
    return path;
  }
  return invoke<string>("worktree_create", { cwd, path, branch });
}

/** AJ-E1002: what removing a worktree would discard. */
export async function worktreeRemovalPreview(path: string): Promise<WorktreeRemovalInfo> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_WORKTREE_REMOVAL__?: WorktreeRemovalInfo };
    if (seams.__AIJ_WORKTREE_REMOVAL__) return seams.__AIJ_WORKTREE_REMOVAL__;
    throw new Error("no worktree here");
  }
  return invoke<WorktreeRemovalInfo>("worktree_removal_preview", { path });
}

/** AJ-E1002: removes a worktree AI Juggler created (typed confirmation
 * when uncommitted work would be discarded). */
export async function worktreeRemove(path: string, confirm: string | null): Promise<void> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_LAUNCH_CALLS__?: string[] };
    seams.__AIJ_LAUNCH_CALLS__ = [...(seams.__AIJ_LAUNCH_CALLS__ ?? []), `worktree-remove:${path}:${confirm ?? ""}`];
    return;
  }
  return invoke<void>("worktree_remove", { path, confirm });
}

/** AJ-E1002: an Instance's git status, read now (on focus). */
export async function gitStatusRefresh(sessionId: string): Promise<GitStatusInfo | null> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_GIT_STATUS__?: Record<string, GitStatusInfo> };
    return seams.__AIJ_GIT_STATUS__?.[sessionId] ?? null;
  }
  return invoke<GitStatusInfo | null>("git_status_refresh", { sessionId });
}

/** What the user chose in the isolation offer. */
export type IsolationDecision =
  | { kind: "worktree"; path: string; branch: string }
  | { kind: "shared" }
  | { kind: "cancel" };

export const ISOLATION_OFFER_EVENT = "aij:isolation-offer";

export type IsolationOfferRequest = {
  offer: IsolationOfferInfo;
  resolve: (decision: IsolationDecision) => void;
};

function askIsolation(offer: IsolationOfferInfo): Promise<IsolationDecision> {
  return new Promise((resolve) => {
    const detail: IsolationOfferRequest & { claimed?: boolean } = { offer, resolve };
    window.dispatchEvent(new CustomEvent(ISOLATION_OFFER_EVENT, { detail }));
    // No dialog mounted: launch nothing rather than share silently.
    window.setTimeout(() => {
      if (!detail.claimed) resolve({ kind: "cancel" });
    }, 0);
  });
}

/** Launches a session — after the AJ-E1001 identity check. A mismatch
 * pauses for the user (switch selection / check again / cancel); there
 * is no "continue anyway", and the backend refuses one regardless. */
export async function launchSession(
  profileId: string,
  cwd: string | null,
  /** AJ-E2304: this session's own runtime ("Start in terminal"), else the profile's. */
  runtime?: "terminal" | "driven",
): Promise<{ session_id: string; pid: number } | null> {
  let recipe = profileId;
  for (;;) {
    const check = await identityCheck(recipe);
    if (check.state !== "mismatch") break;
    const decision = await askIdentityDecision(check);
    if (decision.kind === "cancel") {
      throw new Error(
        `Not launched — «${check.account_label}» is signed in as ${check.detected ?? "someone else"}, not ${check.expected ?? "its account"}.`,
      );
    }
    if (decision.kind === "switch") recipe = decision.recipeId;
  }
  // AJ-E1002: another Instance already works here — offer its own
  // worktree (the default), created only once the user confirms.
  let directory = cwd;
  if (directory) {
    const offer = await isolationOffer(directory);
    if (offer) {
      const decision = await askIsolation(offer);
      if (decision.kind === "cancel") throw new Error("Not launched — cancelled.");
      if (decision.kind === "worktree") directory = await worktreeCreate(directory, decision.path, decision.branch);
    }
  }
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_LAUNCH_CALLS__?: string[]; __AIJ_LAUNCH_CWDS__?: (string | null)[] };
    seams.__AIJ_LAUNCH_CALLS__ = [...(seams.__AIJ_LAUNCH_CALLS__ ?? []), recipe];
    seams.__AIJ_LAUNCH_CWDS__ = [...(seams.__AIJ_LAUNCH_CWDS__ ?? []), directory];
    return null; // preview mode: nothing really launches
  }
  return invoke<{ session_id: string; pid: number }>("launch_session", {
    profileId: recipe,
    cwd: directory,
    runtime: runtime ?? null,
  });
}

/** Resumes the ended session `sessionId` (a new session, the same
 * conversation). In the preview the seam `__AIJ_SESSION_RESUME__` records
 * it. */
export async function sessionResume(sessionId: string): Promise<LaunchResult> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_SESSION_RESUME__?: string[] };
    seams.__AIJ_SESSION_RESUME__ = [...(seams.__AIJ_SESSION_RESUME__ ?? []), sessionId];
    return { session_id: `sess-resumed-${sessionId}`, pid: 0 };
  }
  return invoke<LaunchResult>("session_resume", { sessionId });
}
