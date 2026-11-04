/**
 * AJ-E2911: Report a bug opens from anywhere (Help, your menu, Settings,
 * a failure's screen) into one sheet, mounted once in App. A failure
 * screen passes what it showed; Help and Settings pass nothing.
 */
import { useSyncExternalStore } from "react";
import type { BugContextArgs } from "./ipc/report";

let asked: BugContextArgs | null = null;
const listeners = new Set<() => void>();

const tell = () => {
  for (const listener of listeners) listener();
};

/** What a failure screen showed, for the report. */
export interface Failure {
  from: "switch" | "launch" | "error";
  words?: string | null;
  detail?: string | null;
  code?: string | null;
  atMs?: number | null;
}

/** Opens Report a bug (from Help or Settings without a failure). */
export function openReportBug(failure?: Failure): void {
  asked = failure
    ? { from: failure.from, words: failure.words ?? null, detail: failure.detail ?? null, code: failure.code ?? null, at_ms: failure.atMs ?? null }
    : { from: "help", words: null, detail: null, code: null, at_ms: null };
  tell();
}

/** Closes it. */
export function closeReportBug(): void {
  asked = null;
  tell();
}

/** What Report a bug was opened with, or null while closed. */
export function useReportBug(): BugContextArgs | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => asked,
  );
}
