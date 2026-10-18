import { useEffect, useState } from "react";
import { Card, Chip } from "./primitives";
import type { PreviewFrameInfo, PreviewStateInfo } from "../lib/bridge";

/**
 * The desktop live preview (AJ-E415): a strictly READ-ONLY mirror of a
 * desktop Instance's window — pixels flow app → panel, never back. The
 * only pointer action is click-to-focus (AJ-E414's tiers); no input
 * injection exists here and none may be added (supervision, not remote
 * control — see the task's handover note). Live frames carry the
 * `● Live preview` indicator with their frame age; a minimised/hidden
 * window pauses with the copy-exact text; a missing Screen Recording
 * grant renders the denied state (status/transcript data only below
 * it). The bridge seam is injectable (the E410/focusBridge pattern —
 * a static value import would break vi.mock hoisting for every
 * inspector test).
 */

/** Copy-exact pause text (Req.1, mirrored from the Rust constant). */
export const PAUSED_NOT_VISIBLE = "Preview paused — window not visible";

export type PreviewBridge = {
  start: (sessionId: string) => Promise<void>;
  stop: () => Promise<void>;
  onFrame: (handler: (frame: PreviewFrameInfo) => void) => () => void;
  onState: (handler: (state: PreviewStateInfo) => void) => () => void;
};

const defaultBridge: PreviewBridge = {
  // Dynamic import keeps this module free of static bridge value
  // imports (mock-hoisting safety; every consumer can inject).
  start: (sessionId) => import("../lib/bridge").then((b) => b.previewStart(sessionId)),
  stop: () => import("../lib/bridge").then((b) => b.previewStop()),
  onFrame: (handler) => {
    let dispose: (() => void) | null = null;
    void import("../lib/bridge").then((b) => {
      dispose = b.onPreviewFrame(handler);
    });
    return () => dispose?.();
  },
  onState: (handler) => {
    let dispose: (() => void) | null = null;
    void import("../lib/bridge").then((b) => {
      dispose = b.onPreviewState(handler);
    });
    return () => dispose?.();
  },
};

export function DesktopPreview({
  sessionId,
  enabled,
  visible,
  onFocus,
  now,
  bridge = defaultBridge,
}: {
  sessionId: string;
  /** The Account's preview opt-in (Req.3): off renders nothing here —
   *  the enable lives in Account Actions. */
  enabled: boolean;
  /** True while the panel's tab is shown; hidden tabs PAUSE the stream
   *  (Req.1) but keep the last frame (labelled with its age). */
  visible: boolean;
  /** Click-to-focus (the ONLY pointer action — read-only mirror). */
  onFocus: () => void;
  now: number;
  bridge?: PreviewBridge;
}) {
  const [frame, setFrame] = useState<PreviewFrameInfo | null>(null);
  const [phase, setPhase] = useState<PreviewStateInfo | null>(null);

  useEffect(() => {
    if (!enabled) {
      return;
    }
    const stopFrames = bridge.onFrame((next) => {
      if (next.session_id === sessionId) setFrame(next);
    });
    const stopStates = bridge.onState((next) => {
      if (next.session_id === sessionId) setPhase(next);
    });
    return () => {
      stopFrames();
      stopStates();
    };
  }, [sessionId, enabled, bridge]);

  // The stream runs ONLY while the panel is visible AND enabled
  // (Req.1/4): hidden tabs and unmounts stop it (bounded backend
  // teardown); becoming visible again restarts it.
  useEffect(() => {
    if (!enabled || !visible) {
      return;
    }
    void bridge.start(sessionId).catch(() => {
      /* refused (opt-in off / not desktop): the surface below carries
       * the state honestly */
    });
    return () => {
      void bridge.stop();
    };
  }, [sessionId, enabled, visible, bridge]);

  if (!enabled) {
    return null;
  }

  const ageSec = frame !== null ? Math.max(0, Math.round((now - frame.at_ms) / 1000)) : null;
  return (
    <Card className="flex flex-col gap-2 p-3" data-testid="desktop-preview">
      <div className="flex flex-wrap items-center gap-2">
        {phase?.phase === "live" || (phase === null && frame !== null) ? (
          <span data-testid="preview-live-indicator">
            <Chip tone="live">● Live preview</Chip>
          </span>
        ) : null}
        {ageSec !== null ? (
          <span className="text-[11px] text-faint" data-testid="preview-frame-age">
            frame age {ageSec}s
          </span>
        ) : null}
        <span className="ml-auto text-[10.5px] text-faint">
          read-only mirror · click to focus the real window
        </span>
      </div>
      {phase?.phase === "paused" ? (
        <p className="text-[11.5px] text-muted" data-testid="preview-paused">
          {PAUSED_NOT_VISIBLE}
        </p>
      ) : null}
      {phase?.phase === "denied" ? (
        <div
          className="flex flex-col gap-1.5 rounded-lg border border-warning/25 bg-warning-soft p-2.5"
          data-testid="preview-denied"
        >
          <p className="text-[11.5px] leading-relaxed text-ink">{phase.text}</p>
          <p className="text-[10.5px] text-faint">
            Status and transcript data below keep working without it.
          </p>
        </div>
      ) : null}
      {phase?.phase === "unsupported" ? (
        <p className="text-[11.5px] text-muted" data-testid="preview-unsupported">
          {phase.text}
        </p>
      ) : null}
      {frame !== null && phase?.phase !== "denied" && phase?.phase !== "unsupported" ? (
        <button
          type="button"
          onClick={onFocus}
          title="Focus this instance's window (read-only preview — clicks focus, never inject)"
          className="overflow-hidden rounded-lg border border-hairline outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="preview-frame"
        >
          <img
            src={`data:image/png;base64,${frame.png_b64}`}
            alt={`Live preview of this instance's window (captured ${ageSec ?? 0}s ago)`}
            className={
              "block w-full " + (phase?.phase === "paused" ? "opacity-40" : "opacity-100")
            }
          />
        </button>
      ) : phase?.phase === "live" ? (
        <p className="text-[11px] text-faint" data-testid="preview-connecting">
          Connecting the preview…
        </p>
      ) : null}
    </Card>
  );
}
