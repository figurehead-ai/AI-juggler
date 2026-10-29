/**
 * AJ-E2501 — the window's view of the one notification model: its summary
 * (the bell's count, the centre's header), kept current by the backend's
 * event, and whether the centre is open. Every list under `notices:` is
 * read again when the model's version moves. A write's answer is the
 * summary, applied at once (`acted`).
 */
import { useEffect, useSyncExternalStore } from "react";
import { refreshPaged } from "./paged";
import { notificationsSummary, onNoticesChanged, type NoticeSummaryInfo } from "./ipc/notices";
import { showToast } from "../components/Toast";
import { errorText } from "./bridge";

let summary: NoticeSummaryInfo | null = null;
let centreOpen = false;
let started = false;
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** A new summary: the views follow, the lists are read again if it moved. */
export function applySummary(next: NoticeSummaryInfo): void {
  const moved = summary?.version !== next.version;
  summary = next;
  emit();
  if (moved) refreshPaged("notices:");
}

/** Starts listening (once): the first read, then every change. */
export function startNotices(): void {
  if (started) return;
  started = true;
  void notificationsSummary().then(applySummary, () => null);
  onNoticesChanged(applySummary);
}

/** Test seam: forget the summary and the listening. */
export function resetNotices(): void {
  summary = null;
  centreOpen = false;
  started = false;
  emit();
}

/** Runs a write; its answer (the summary) is applied, a failure said
 * with Retry (an error toast stays until it's closed). */
export async function acted(write: () => Promise<NoticeSummaryInfo>, failed = "That didn't take"): Promise<boolean> {
  try {
    applySummary(await write());
    return true;
  } catch (cause) {
    showToast({ id: "notices-failed", tone: "error", message: failed, detail: errorText(cause), action: { label: "Retry", run: () => void acted(write, failed) } });
    return false;
  }
}

const readSummary = () => summary;
const readOpen = () => centreOpen;

/** The model's summary (null until the first read answers). */
export function useNoticeSummary(): NoticeSummaryInfo | null {
  useEffect(startNotices, []);
  return useSyncExternalStore(subscribe, readSummary, readSummary);
}

/** The centre: open, closed, or the other. */
export function setCentreOpen(open: boolean): void {
  if (centreOpen === open) return;
  centreOpen = open;
  emit();
}

export function toggleCentre(): void {
  setCentreOpen(!centreOpen);
}

export function useCentreOpen(): boolean {
  return useSyncExternalStore(subscribe, readOpen, readOpen);
}
