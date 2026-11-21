/**
 * AJ-E1002 — an Instance's git state in the inspector (read fresh when
 * the Instance is focused — the rail uses the 30 s cache), and removing
 * a worktree AI Juggler created. Removal never discards uncommitted work
 * silently: the dialog says exactly what would be lost and, when
 * anything would be, needs the folder's name typed back. The branch is
 * always kept.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { errorText } from "../lib/bridge";
import { gitStatusRefresh, worktreeRemovalPreview, worktreeRemove } from "../lib/ipc/launch";
import type { GitStatusInfo } from "../types/generated/GitStatusInfo";
import type { WorktreeRemovalInfo } from "../types/generated/WorktreeRemovalInfo";
import { DialogFrame, DialogOverlay } from "./DialogFrame";
import { Button } from "./primitives";

function words(status: GitStatusInfo): string {
  const parts = [`branch ${status.branch ?? "detached"}`];
  const dirty = status.changed + status.untracked;
  parts.push(dirty === 0 ? "nothing uncommitted" : `${dirty} uncommitted ${dirty === 1 ? "file" : "files"}`);
  if (status.ahead !== null) parts.push(`${status.ahead} ahead`);
  if (status.behind !== null) parts.push(`${status.behind} behind`);
  if (status.worktree) parts.push("own worktree");
  return parts.join(" · ");
}

export function WorktreeGit({ sessionId, live }: { sessionId: string; live: boolean }): JSX.Element | null {
  const [status, setStatus] = useState<GitStatusInfo | null>(null);
  const [removing, setRemoving] = useState(false);
  const [tick, setTick] = useState(0);
  useEffect(() => {
    let alive = true;
    const read = () =>
      gitStatusRefresh(sessionId)
        .then((next) => alive && setStatus(next))
        .catch(() => alive && setStatus(null));
    void read();
    // Refreshed on focus and at most every 30 s while shown.
    const timer = window.setInterval(() => void read(), 30_000);
    window.addEventListener("focus", read);
    return () => {
      alive = false;
      window.clearInterval(timer);
      window.removeEventListener("focus", read);
    };
  }, [sessionId, tick]);
  if (!status) return null;
  return (
    <div className="flex flex-wrap items-center gap-2 text-label text-muted" data-testid="instance-git">
      <span className="text-ink">{words(status)}</span>
      {status.created_by_juggler && !live && (
        <button
          type="button"
          onClick={() => setRemoving(true)}
          className="rounded border border-hairline px-1.5 py-0.5 text-label text-ink hover:border-danger/50"
          data-testid="worktree-remove"
        >
          Remove worktree…
        </button>
      )}
      {removing && (
        <WorktreeRemoveDialog
          path={status.path}
          onClose={() => {
            setRemoving(false);
            setTick((value) => value + 1);
          }}
        />
      )}
    </div>
  );
}

export function WorktreeRemoveDialog({ path, onClose }: { path: string; onClose: () => void }): JSX.Element {
  const [info, setInfo] = useState<WorktreeRemovalInfo | null>(null);
  const [typed, setTyped] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    worktreeRemovalPreview(path)
      .then(setInfo)
      .catch((cause) => setError(errorText(cause)));
  }, [path]);
  useEffect(() => {
    dialogRef.current?.querySelector<HTMLElement>("input, button")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [info, onClose]);
  const armed = info !== null && !info.in_use && (!info.needs_confirmation || typed === info.folder);
  const remove = async () => {
    if (!info) return;
    try {
      await worktreeRemove(path, info.needs_confirmation ? typed : null);
      setDone(true);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  return (
    <DialogOverlay className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <DialogFrame
        frameRef={dialogRef}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="worktree-remove-title"
        aria-describedby="worktree-remove-summary"
        data-testid="worktree-remove-dialog"
        className="w-full max-w-md rounded-xl border border-hairline bg-surface shadow-card"
        headerClassName="px-5 pt-5"
        bodyClassName="flex flex-col gap-3 px-5 pt-3 pb-3"
        header={
          <h3 id="worktree-remove-title" className="text-sm font-semibold">
            Remove this worktree?
          </h3>
        }
        footerClassName="px-5 pt-3 pb-5"
        footer={
          done ? (
            <div className="flex justify-end">
              <p className="flex-1 text-label text-ink" aria-live="polite">
                Removed. The branch {info?.branch} stays.
              </p>
              <Button size="sm" variant="secondary" onClick={onClose}>
                Close
              </Button>
            </div>
          ) : (
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={onClose}>
                Keep it
              </Button>
              <Button
                size="sm"
                variant="danger"
                disabled={!armed}
                onClick={() => void remove()}
                data-testid="worktree-remove-go"
              >
                {info?.needs_confirmation ? "Discard and remove" : "Remove"}
              </Button>
            </div>
          )
        }
      >
        {info && (
          <>
            <p className="break-all font-mono text-label text-muted">{info.path}</p>
            <p id="worktree-remove-summary" className="text-label text-ink" data-testid="worktree-remove-summary">
              {info.needs_confirmation && <span aria-hidden="true">⚠ </span>}
              {info.summary}
            </p>
            {info.in_use && (
              <p className="text-label text-ink">A session still runs in it — stop it first.</p>
            )}
            {info.needs_confirmation && !info.in_use && (
              <label className="flex flex-col gap-1 text-label text-ink">
                Type «{info.folder}» to discard them
                <input autoCorrect="off" autoCapitalize="off" spellCheck={false}
                  type="text"
                  value={typed}
                  onChange={(event) => setTyped(event.target.value)}
                  className="rounded border border-hairline bg-canvas px-2 py-1 font-mono text-label text-ink"
                  data-testid="worktree-remove-confirm"
                  aria-label={`Type ${info.folder} to confirm`}
                />
              </label>
            )}
          </>
        )}
        {error && (
          <p role="alert" className="text-label text-ink">
            {error}
          </p>
        )}
      </DialogFrame>
    </DialogOverlay>
  );
}
