/**
 * AJ-E1204 — the recovery code, shown when Cloud backup is first turned
 * on. It is the only way a new Mac can open the backups, so the sheet
 * shows it as selectable text with Copy and Save, says plainly what
 * losing it means, and asks for "I've saved my recovery code" before
 * any backup is uploaded. Once confirmed, it is never shown again.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { errorText } from "../lib/bridge";
import { vaultConfirmSaved, vaultSaveCode, vaultSetup } from "../lib/ipc/vault";
import { Checkbox } from "./controls";
import { DialogFrame, DialogOverlay } from "./DialogFrame";
import { Button } from "./primitives";

export function RecoveryCodeSheet({ onClose }: { onClose: (saved: boolean) => void }): JSX.Element {
  const [code, setCode] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const codeRef = useRef<HTMLParagraphElement | null>(null);
  useEffect(() => {
    vaultSetup()
      .then((info) => setCode(info.code))
      .catch((cause) => setError(errorText(cause)));
  }, []);
  useEffect(() => {
    codeRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [code, onClose]);
  const copy = async () => {
    if (!code) return;
    try {
      await navigator.clipboard.writeText(code);
      setNote("Copied. Paste it somewhere safe, then clear your clipboard.");
    } catch {
      setNote("Couldn't copy — select the code and copy it by hand.");
    }
  };
  const save = async () => {
    try {
      if (await vaultSaveCode()) setNote("Saved to the file you chose.");
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  const done = async () => {
    try {
      await vaultConfirmSaved();
      onClose(true);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  return (
    <DialogOverlay className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <DialogFrame
        role="dialog"
        aria-modal="true"
        aria-labelledby="recovery-title"
        aria-describedby="recovery-warning"
        data-testid="recovery-sheet"
        className="w-full max-w-lg rounded-xl border border-hairline bg-surface shadow-card"
        headerClassName="px-5 pt-5"
        bodyClassName="flex flex-col gap-3 px-5 pt-3 pb-3"
        footerClassName="px-5 pt-3 pb-5"
        header={
          <h3 id="recovery-title" className="text-sm font-semibold">
            Save your recovery code
          </h3>
        }
        footer={
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="ghost" onClick={() => onClose(false)}>
              Later — no backups until then
            </Button>
            <Button size="sm" variant="primary" disabled={!saved || !code} onClick={() => void done()} data-testid="recovery-done">
              Start backups
            </Button>
          </div>
        }
      >
        <p className="text-label text-ink">
          Your Cloud backups are encrypted on this computer with a key only you have. Figurehead can't read them. This code is
          that key: you'll need it to open your backups on a new Mac.
        </p>
        <p
          ref={codeRef}
          tabIndex={0}
          aria-label="Recovery code"
          className="select-all break-all rounded-lg border border-hairline bg-canvas px-3 py-2 text-center font-mono text-meta leading-relaxed tracking-wide text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="recovery-code"
        >
          {code
            ? code.split("-").map((group, index, all) => (
                // Each group (and its dash) stays on one line.
                <span key={index} className="inline-block whitespace-nowrap">
                  {group}
                  {index < all.length - 1 ? "-" : ""}
                </span>
              ))
            : "…"}
        </p>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => void copy()}
            disabled={!code}
            className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
            data-testid="recovery-copy"
          >
            Copy
          </button>
          <button
            type="button"
            onClick={() => void save()}
            disabled={!code}
            className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
            data-testid="recovery-save"
          >
            Save…
          </button>
          <span className="self-center text-label text-muted" aria-live="polite">
            {note}
          </span>
        </div>
        <p
          id="recovery-warning"
          className="rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-label font-medium text-ink"
          data-testid="recovery-warning"
        >
          <span aria-hidden="true">⚠ </span>Without this code, a new Mac can't open your backups. We can't recover it for you.
        </p>
        <label className="flex items-center gap-2 text-label text-ink">
          <Checkbox
            checked={saved}
            onChange={(event) => setSaved(event.target.checked)}
            data-testid="recovery-confirm"
          />
          I've saved my recovery code
        </label>
        {error && (
          <p role="alert" className="text-label text-ink">
            {error}
          </p>
        )}
      </DialogFrame>
    </DialogOverlay>
  );
}
