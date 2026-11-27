/**
 * D-136 point 5 — the parts of Bot's confirm cards (a found row's
 * "Watch…", AJ-E3815; a request code fully read, AJ-E3830's Beta): the
 * card with its eyebrow and close, the echo the code writes, quiet hours
 * in one line, Watch / Change… / Cancel, the "acts only" line, and the
 * line once it is set. One look for both cards (ui-consistency).
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";
import { IconClose } from "../icons-controls";
import { IconShield } from "../icons";
import { IconEye, IconMoon } from "./bot-icons";
import { WORDS } from "../../lib/bot-words";
import { notificationPrefsGet } from "../../lib/ipc/settings";

function hour(h: number): string {
  return `${String(h).padStart(2, "0")}:00`;
}

/** The user's quiet hours in one line ("Between 22:00 and 07:00 it waits
 * till morning."), or null with none. */
export function useQuietLine(): string | null {
  const [quiet, setQuiet] = useState<string | null>(null);
  useEffect(() => {
    void notificationPrefsGet()
      .then((prefs) => setQuiet(prefs.quiet_start === prefs.quiet_end ? null : `Between ${hour(prefs.quiet_start)} and ${hour(prefs.quiet_end)} it waits till morning.`))
      .catch(() => setQuiet(null));
  }, []);
  return quiet;
}

/** The card: its eyebrow, its close, and what it holds. */
export function ConfirmShell({ label, testId, onClose, children, cardRef }: { label: string; testId: string; onClose: () => void; children: ReactNode; cardRef?: (node: HTMLElement | null) => void }): JSX.Element {
  return (
    <section ref={cardRef} aria-label={label} className="flex flex-col gap-3 rounded-[14px] bg-[color-mix(in_srgb,var(--color-accent)_6%,var(--color-surface))] p-3.5 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-[color-mix(in_srgb,var(--color-accent)_9%,var(--color-surface))]" data-testid={testId}>
      <div className="flex items-center gap-2">
        <span className="grow text-label font-semibold tracking-[.07em] text-accent-hi uppercase">{WORDS.confirmEyebrow}</span>
        <button type="button" className="grid size-6 place-items-center rounded-lg text-muted outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" title={WORDS.cancel} aria-label={WORDS.cancel} onClick={onClose}>
          <IconClose width={14} height={14} aria-hidden="true" />
        </button>
      </div>
      {children}
    </section>
  );
}

/** The echo in plain words, the code's own. */
export function EchoLine({ words, testId = "bot-watch-echo" }: { words: string; testId?: string }): JSX.Element {
  return (
    <p className="rounded-[10px] bg-surface px-3 py-2.5 text-[15px] leading-normal text-ink-2 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" aria-live="polite" data-testid={testId}>
      {words}
    </p>
  );
}

/** Quiet hours, the problem and any error, under the echo. */
export function ConfirmNotes({ problem, quiet, error }: { problem: string | null; quiet: string | null; error: string | null }): JSX.Element {
  return (
    <>
      {problem && <p className="text-meta text-danger">{problem}</p>}
      {quiet && (
        <span className="flex items-center gap-2 text-meta text-muted">
          <IconMoon width={14} height={14} aria-hidden="true" />
          {quiet}
        </span>
      )}
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </>
  );
}

/** Watch / Change… / Cancel, then the "acts only" line. */
export function ConfirmFoot({ canWatch, onWatch, onChange, changeExpanded, onCancel, testPrefix = "bot-watch" }: { canWatch: boolean; onWatch: () => void; onChange: () => void; changeExpanded?: boolean; onCancel: () => void; testPrefix?: string }): JSX.Element {
  return (
    <>
      <div className="flex items-center gap-2">
        <button type="button" disabled={!canWatch} className="inline-flex h-7 items-center gap-1.5 rounded-lg bg-accent-fill px-2.5 text-meta font-medium text-white outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-50 disabled:shadow-none" onClick={onWatch} data-testid={`${testPrefix}-confirm`}>
          <IconEye width={14} height={14} aria-hidden="true" />
          {WORDS.watchButton}
        </button>
        <button type="button" aria-expanded={changeExpanded} className="inline-flex h-7 items-center rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={onChange} data-testid={`${testPrefix}-change`}>
          {WORDS.change}
        </button>
        <button type="button" className="inline-flex h-7 items-center rounded-lg px-2.5 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={onCancel} data-testid={`${testPrefix}-cancel`}>
          {WORDS.cancel}
        </button>
      </div>
      <span className="flex items-center gap-1.5 text-label text-muted">
        <IconShield width={13} height={13} aria-hidden="true" />
        {WORDS.actsOnly}
      </span>
    </>
  );
}

/** The line once the watch is set. */
export function WatchSet({ words, testId = "bot-watch-done" }: { words: string; testId?: string }): JSX.Element {
  return (
    <output className="flex items-center gap-2.5 rounded-[14px] bg-surface px-3.5 py-3 text-body text-ink-2 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid={testId}>
      <IconEye width={16} height={16} aria-hidden="true" className="shrink-0 text-accent-hi" />
      {WORDS.watchSet(words)}
    </output>
  );
}
