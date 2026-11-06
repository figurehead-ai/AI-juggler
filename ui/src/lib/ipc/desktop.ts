/**
 * Desktop apps: launch and focus, and the live preview (AJ-E415).
 */
import { invoke, listen, liveBackend as hasBackend } from "./transport";
import { EVENTS } from "../bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { DesktopLaunchInfo } from "../../types/generated/DesktopLaunchInfo";
import type { FocusOutcomeInfo } from "../../types/generated/FocusOutcomeInfo";
import type { PreviewFrameInfo } from "../../types/generated/PreviewFrameInfo";
import type { PreviewStateInfo } from "../../types/generated/PreviewStateInfo";
import { previewHooks } from "../preview-hooks";

/** AJ-E413: launch the Provider's desktop app under an Account. */
export async function desktopLaunch(accountId: string): Promise<DesktopLaunchInfo> {
  if (!hasBackend) {
    return {
      session_id: "sess-desktop-1",
      pid: 42_042,
      bundle_id: "dev.aijuggler.fake-desktop",
      isolation_label: "Single-account app — Juggler tracks, cannot isolate",
    };
  }
  return invoke<DesktopLaunchInfo>("desktop_launch", { accountId });
}

/** AJ-E414: run the focus tiers for one desktop Instance — activate,
 * match the recorded window identity, clear AXMinimized, AXRaise. The
 * returned outcome says exactly which tier ran (never overstated). */
export async function desktopFocus(sessionId: string): Promise<FocusOutcomeInfo> {
  if (!hasBackend) {
    // Harness seam (AJ-E802): captures may pin the outcome they render.
    const w = window as unknown as { __AIJ_FOCUS_OUTCOME__?: FocusOutcomeInfo };
    return (
      w.__AIJ_FOCUS_OUTCOME__ ?? {
        outcome: "Focused window",
        permission_missing: false,
        window_title: "notes-a",
      }
    );
  }
  return invoke<FocusOutcomeInfo>("desktop_focus", { sessionId });
}


/** AJ-E415: starts the read-only live preview for the selected desktop
 * Instance (requires the Account's opt-in; one at a time). */
export async function previewStart(sessionId: string): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("preview_start", { sessionId });
}

/** AJ-E415: stops the live preview (bounded teardown <1 s). */
export async function previewStop(): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("preview_stop");
}

/** AJ-E415: sets the per-Account live-preview opt-in (the Screen
 * Recording consent moment; off by default). */
export async function accountSetPreviewOptIn(
  accountId: string,
  optIn: boolean,
): Promise<AccountInfo> {
  if (!hasBackend) {
    return {
      ...(previewHooks().fixtures.MOCK_ACCOUNTS.find((a) => a.id === accountId) ?? previewHooks().fixtures.MOCK_ACCOUNTS[0]!),
      preview_opt_in: optIn,
    };
  }
  return invoke<AccountInfo>("account_set_preview_opt_in", { accountId, optIn });
}

/** AJ-E415: live frames (filtered to one session by the caller). */
export function onPreviewFrame(
  handler: (frame: PreviewFrameInfo) => void,
): () => void {
  if (!hasBackend) {
    // Harness mode: emit one deterministic frame immediately so the
    // panel (and captures) render a real preview; the phase comes from
    // onPreviewState below.
    handler({
      session_id: "sess-desktop-1",
      png_b64: previewHooks().appState.MOCK_PREVIEW_PNG_B64,
      at_ms: Date.now(),
      seq: 0,
    });
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<PreviewFrameInfo>(EVENTS.previewFrame, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {
      /* backend gone; the panel shows its paused state */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** AJ-E415: phase changes (live | paused | denied | unsupported). */
export function onPreviewState(
  handler: (state: PreviewStateInfo) => void,
): () => void {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_PREVIEW_PHASE__?: PreviewStateInfo };
    const seeded = w.__AIJ_PREVIEW_PHASE__;
    if (seeded) {
      handler(seeded);
    } else {
      handler({
        session_id: "sess-desktop-1",
        phase: "live",
        text: "",
      });
    }
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<PreviewStateInfo>(EVENTS.previewState, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {
      /* backend gone */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}
