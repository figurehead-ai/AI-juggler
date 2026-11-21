/**
 * AJ-E3812 — the Models screen's two reads, shared and kept: the screen's
 * list (this Mac, the recommendation, the models it can use) and the
 * models on this Mac with the download. Lazy: nothing is asked until the
 * screen opens. Cached: opened again, it paints the last answer at once
 * and reads again behind it. Kept current by the backend's events
 * (`ai-models://changed`, and `ai://jobs-changed` for the recommendation),
 * never by a timer; a running download's steps arrive as they happen.
 */
import { useEffect, useSyncExternalStore } from "react";
import { sharedRead, useSharedRead, whenForgotten } from "./shared-read";
import { onAiJobsChanged } from "./ipc/ai";
import { aiModelsCatalog, aiModelsInstalled, onAiModelsChanged, onAiModelsProgress, type DownloadInfo, type ModelsCatalog, type ModelsInfo } from "./ipc/models";

/** Kept until the backend says it changed (read again on each open). */
const KEPT = 0;

const CATALOG = sharedRead(aiModelsCatalog);
const INSTALLED = sharedRead(aiModelsInstalled);

/** The running download's last step, from its progress events. */
let step: DownloadInfo | null = null;
const steppers = new Set<() => void>();

let stop: (() => void) | null = null;
function listen(): void {
  if (stop) return;
  const changed = onAiModelsChanged(() => {
    step = null;
    void INSTALLED.refresh();
    void CATALOG.refresh();
  });
  const jobs = onAiJobsChanged(() => void CATALOG.refresh());
  const progress = onAiModelsProgress((info) => {
    step = info;
    for (const listener of steppers) listener();
  });
  stop = () => {
    changed();
    jobs();
    progress();
  };
  whenForgotten(() => {
    stop?.();
    stop = null;
    step = null;
  });
}

/** The screen's list (`null` until first read). */
export function useModelsCatalog(): ModelsCatalog | null {
  useEffect(listen, []);
  return useSharedRead(CATALOG, { maxAgeMs: KEPT }) ?? null;
}

/** The models on this Mac and the download (`null` until first read),
 * the download at its latest step. */
export function useModelsInstalled(): ModelsInfo | null {
  useEffect(listen, []);
  const info = useSharedRead(INSTALLED, { maxAgeMs: KEPT }) ?? null;
  const latest = useSyncExternalStore(
    (listener) => {
      steppers.add(listener);
      return () => steppers.delete(listener);
    },
    () => step,
  );
  if (!info || !latest || info.download?.id !== latest.id) return info;
  return { ...info, download: latest };
}

/** Reads both again now (Check again; after a click whose answer the
 * backend doesn't announce). */
export function refreshModels(): void {
  void CATALOG.refresh();
  void INSTALLED.refresh();
}
