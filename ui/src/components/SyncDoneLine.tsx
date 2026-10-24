/**
 * AJ-E1507: what a sync change did, with its Undo (the set-up record's,
 * so Settings › Context packs & secrets can undo it later too).
 */
import { useEffect, useRef, useState } from "react";
import { errorText } from "../lib/bridge";
import { switchSetupUndo } from "../lib/ipc/switchSetup";

/** What a change did, with its record for Undo. */
export type Done = { message: string; record: string | null };

/** The sync sheet's section headings. */
export const SYNC_HEADING = "text-[11px] uppercase tracking-wider text-muted";

/** What a change did, with its Undo. */
export function DoneLine({ done, onUndone }: { done: Done; onUndone: () => void }) {
  const [status, setStatus] = useState(done.message);
  const [error, setError] = useState<string | null>(null);
  const [undone, setUndone] = useState(false);
  // The button that made the change has gone (its row or panel): focus
  // comes here, so the keyboard keeps its place and Escape still closes
  // the sheet.
  const line = useRef<HTMLParagraphElement | null>(null);
  useEffect(() => line.current?.focus(), []);
  const undo = async () => {
    if (!done.record) return;
    setError(null);
    try {
      setStatus(await switchSetupUndo(done.record));
      setUndone(true);
      line.current?.focus();
      onUndone();
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  return (
    <div className="flex flex-col gap-1 text-xs">
      <p ref={line} tabIndex={-1} className="flex items-center gap-2 outline-none" role="status">
        <span className="text-ink">{status}</span>
        {done.record && !undone && (
          <button type="button" onClick={() => void undo()} className="rounded border border-hairline px-1.5 py-0.5 text-[10px] hover:bg-surface-2">
            Undo
          </button>
        )}
      </p>
      {!undone && done.record && <p className="text-[11px] text-muted">You can also undo it later in Settings › Switching.</p>}
      {error && (
        <p className="text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
