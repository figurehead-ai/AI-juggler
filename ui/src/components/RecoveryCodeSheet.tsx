/**
 * AJ-E1204 — the recovery code, shown when Cloud backup is first turned
 * on. It is the only way a new Mac can open the backups, so the sheet
 * shows it as selectable text with Copy and Save, says plainly what
 * losing it means, and asks for "I've saved my recovery code" before
 * any backup is uploaded. Once confirmed, it is never shown again.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { errorText, vaultConfirmSaved, vaultSaveCode, vaultSetup } from "../lib/bridge";

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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="recovery-title"
        aria-describedby="recovery-warning"
        data-testid="recovery-sheet"
        className="flex w-full max-w-lg flex-col gap-3 rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        <h3 id="recovery-title" className="text-sm font-semibold">
          Save your recovery code
        </h3>
        <p className="text-[11px] text-ink">
          Your Cloud backups are encrypted on this Mac with a key only you have. Figurehead can't read them. This code is
          that key: you'll need it to open your backups on a new Mac.
        </p>
        <p
          ref={codeRef}
          tabIndex={0}
          aria-label="Recovery code"
          className="select-all break-all rounded-lg border border-hairline bg-canvas px-3 py-2 text-center font-mono text-[13px] leading-relaxed tracking-wide text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
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
          <span className="self-center text-[11px] text-muted" aria-live="polite">
            {note}
          </span>
        </div>
        <p
          id="recovery-warning"
          className="rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-[11px] font-medium text-ink"
          data-testid="recovery-warning"
        >
          <span aria-hidden="true">⚠ </span>Without this code, a new Mac can't open your backups. We can't recover it for you.
        </p>
        <label className="flex items-center gap-2 text-[11px] text-ink">
          <input
            type="checkbox"
            checked={saved}
            onChange={(event) => setSaved(event.target.checked)}
            data-testid="recovery-confirm"
          />
          I've saved my recovery code
        </label>
        {error && (
          <p role="alert" className="text-[11px] text-ink">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <button type="button" onClick={() => onClose(false)} className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink">
            Later — no backups until then
          </button>
          <button
            type="button"
            disabled={!saved || !code}
            onClick={() => void done()}
            className={
              "rounded-lg border px-3 py-1.5 text-xs font-medium " +
              (saved && code ? "border-accent/60 text-ink" : "border-hairline text-muted")
            }
            data-testid="recovery-done"
          >
            Start backups
          </button>
        </div>
      </div>
    </div>
  );
}
