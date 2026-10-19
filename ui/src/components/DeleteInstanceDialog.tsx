/**
 * AJ-E1007 — deleting an Instance is separate from archiving, and there
 * is no single-click path to it: the dialog first reads exactly what
 * would be removed and what stays, and the button only arms once the
 * word "delete" is typed. The backend also refuses without the preview's
 * one-time token.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { errorText, instanceDelete, instanceDeletePreview, type DeletePreviewInfo } from "../lib/bridge";

export function DeleteInstanceDialog({
  sessionId,
  onClose,
}: {
  sessionId: string;
  onClose: (deleted: boolean) => void;
}): JSX.Element {
  const [preview, setPreview] = useState<DeletePreviewInfo | null>(null);
  const [typed, setTyped] = useState("");
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    instanceDeletePreview(sessionId)
      .then(setPreview)
      .catch((cause) => setError(errorText(cause)));
  }, [sessionId]);
  useEffect(() => {
    dialogRef.current?.querySelector<HTMLElement>("input, button")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [preview, onClose]);
  const armed = preview !== null && preview.token !== null && typed.trim().toLowerCase() === "delete";
  const run = async () => {
    if (!preview?.token) return;
    try {
      await instanceDelete(sessionId, preview.token);
      onClose(true);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="delete-title"
        aria-describedby="delete-removes"
        data-testid="delete-dialog"
        className="flex w-full max-w-lg flex-col gap-3 rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        <h3 id="delete-title" className="text-sm font-semibold">
          Delete this session{preview?.account_label ? ` (${preview.account_label})` : ""}?
        </h3>
        {preview?.refused ? (
          <p className="text-[11px] text-ink" data-testid="delete-refused">
            <span aria-hidden="true">⚠ </span>
            {preview.refused}
          </p>
        ) : (
          preview && (
            <>
              <div id="delete-removes" className="text-[11px] text-ink">
                <p className="font-medium">This cannot be undone. It permanently removes:</p>
                <ul className="ml-4 list-disc" data-testid="delete-removes">
                  {preview.removes.map((line) => (
                    <li key={line}>{line}</li>
                  ))}
                </ul>
              </div>
              <div className="text-[11px] text-muted">
                <p className="font-medium text-ink">It keeps:</p>
                <ul className="ml-4 list-disc break-all" data-testid="delete-keeps">
                  {preview.keeps.map((line) => (
                    <li key={line}>{line}</li>
                  ))}
                </ul>
              </div>
              <label className="flex flex-col gap-1 text-[11px] text-ink">
                Type «delete» to confirm
                <input
                  type="text"
                  value={typed}
                  onChange={(event) => setTyped(event.target.value)}
                  className="rounded border border-hairline bg-canvas px-2 py-1 font-mono text-[11px] text-ink"
                  aria-label="Type delete to confirm"
                  data-testid="delete-confirm"
                />
              </label>
            </>
          )
        )}
        {error && (
          <p role="alert" className="text-[11px] text-ink">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <button type="button" onClick={() => onClose(false)} className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink">
            Keep it
          </button>
          {!preview?.refused && (
            <button
              type="button"
              disabled={!armed}
              onClick={() => void run()}
              className={
                "rounded-lg border px-3 py-1.5 text-xs " +
                (armed ? "border-danger/60 text-ink hover:border-danger" : "border-hairline text-muted")
              }
              data-testid="delete-go"
            >
              Delete permanently
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
