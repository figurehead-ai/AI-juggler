/**
 * AJ-E1530 Req.1 (ADR-0018 amendment, opt-in): measuring limits for an
 * account's sessions started outside AI Juggler. The preview and the
 * tests read the seam `__AIJ_CAPTURE__` (per account) instead.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { CaptureOffInfo } from "../../types/generated/CaptureOffInfo";
import type { CapturePreviewInfo } from "../../types/generated/CapturePreviewInfo";
import type { CaptureStatusInfo } from "../../types/generated/CaptureStatusInfo";

export type { CaptureOffInfo, CapturePreviewInfo, CaptureStatusInfo };

type Seam = Record<string, { status: CaptureStatusInfo; preview?: CapturePreviewInfo; refuse?: string }>;

const seam = () => (window as unknown as { __AIJ_CAPTURE__?: Seam }).__AIJ_CAPTURE__ ?? {};

/** The preview's stand-in: an account used where it is, off. */
function mockEntry(accountId: string) {
  const all = seam();
  all[accountId] ??= { status: { offered: true, state: "off", file: "~/.claude/settings.json", reason: null } };
  (window as unknown as { __AIJ_CAPTURE__?: Seam }).__AIJ_CAPTURE__ = all;
  return all[accountId]!;
}

/** Where the account's capture stands. */
export async function captureStatus(accountId: string): Promise<CaptureStatusInfo> {
  if (!hasLiveBackend()) return mockEntry(accountId).status;
  return invoke<CaptureStatusInfo>("quota_capture_status", { accountId });
}

/** The exact change, before anything is written. */
export async function capturePreview(accountId: string): Promise<CapturePreviewInfo> {
  if (!hasLiveBackend()) {
    const entry = mockEntry(accountId);
    if (entry.refuse) throw new Error(entry.refuse);
    return (
      entry.preview ?? {
        file: entry.status.file ?? "~/.claude/settings.json",
        before: '"statusLine": {"type": "command", "command": "~/bin/my-line.sh"}',
        after: `"statusLine": {"type":"command","command":"'~/Library/Application Support/AI Juggler/overlays/${accountId}/juggler-statusline.sh'"}`,
        fingerprint: "preview",
      }
    );
  }
  return invoke<CapturePreviewInfo>("quota_capture_preview", { accountId });
}

/** Turns it on, as previewed (`fingerprint` from the preview). */
export async function captureTurnOn(accountId: string, fingerprint: string): Promise<void> {
  if (!hasLiveBackend()) {
    mockEntry(accountId).status = { ...mockEntry(accountId).status, state: "on" };
    return;
  }
  await invoke("quota_capture_turn_on", { accountId, fingerprint });
}

/** Turns it off. */
export async function captureTurnOff(accountId: string): Promise<CaptureOffInfo> {
  if (!hasLiveBackend()) {
    const entry = mockEntry(accountId);
    const outcome = entry.status.state === "changed" ? "theirs" : "restored";
    entry.status = { ...entry.status, state: "off" };
    return { outcome };
  }
  return invoke<CaptureOffInfo>("quota_capture_turn_off", { accountId });
}
