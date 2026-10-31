/**
 * 0.3.2 — "Continue on another account…" opens the session's ranked
 * accounts in a dialog, over whatever page is showing (the maintainer's
 * 0.3.0 walk: the banner's button only opened the session's page, and
 * the list it focused sat unnoticed in the side column). One dialog for
 * a session AI Juggler runs and one another app runs; its body is the
 * page's own switch list, moved here while the dialog is open, so the two
 * can't drift. Picking an account opens the switch screen, as from the
 * page; Esc, × or a click outside close it and change nothing.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { noteSwitchOpener } from "../lib/handover-focus";
import { useEscapeCloses } from "../lib/use-escape-closes";

/** The pickable accounts in either switch list. */
const TARGETS = "[data-testid='continue-target']:not(:disabled), button[data-row]:not(:disabled)";
/** Any pick that opens a switch: an account here or on another Mac. */
const PICKS = `${TARGETS}, [data-testid='remote-switch-target']:not(:disabled)`;
/** The Suggested one, in either list. */
const SUGGESTED = "[data-testid='continue-target'][data-suggested='true']:not(:disabled), button[data-row][aria-pressed='true']:not(:disabled)";

export function ContinueDialog({ title, detail, onClose: close, children }: { title: string; detail?: string; onClose: () => void; children: ReactNode }): JSX.Element {
  const ref = useRef<HTMLDivElement | null>(null);
  // WCAG 2.4.3: the control that opened the dialog gets the keyboard
  // back, whether it closes or an account is picked. A pick hands it back
  // first, so the switch it opens returns there on Cancel (its rows here
  // are gone by then: 0.3.2's keyboard walk ended on the page's body).
  const [opener] = useState(() => document.activeElement);
  const giveBack = () => {
    if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
  };
  const onClose = () => {
    close();
    requestAnimationFrame(giveBack);
  };
  useEscapeCloses(onClose);
  // The Suggested account takes the keyboard, else the first that can be
  // picked, else the dialog; again once the ranked list arrives.
  useEffect(() => {
    const place = () => {
      const box = ref.current;
      if (!box) return false;
      const active = document.activeElement;
      // Focus the user moved inside stays where they put it.
      if (active && active !== box && box.contains(active)) return true;
      const best = box.querySelector<HTMLElement>(SUGGESTED) ?? box.querySelector<HTMLElement>(TARGETS);
      (best ?? box).focus();
      return best !== null;
    };
    if (place()) return;
    const watch = new MutationObserver(() => place() && watch.disconnect());
    if (ref.current) watch.observe(ref.current, { childList: true, subtree: true });
    return () => watch.disconnect();
  }, []);
  return createPortal(
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-6 pt-[12vh]" onMouseDown={(event) => event.target === event.currentTarget && onClose()} data-testid="continue-dialog-backdrop">
      <div ref={ref} role="dialog" aria-modal="true" aria-labelledby="continue-dialog-title" tabIndex={-1} data-testid="continue-dialog" onClickCapture={(event) => (event.target as Element).closest(PICKS) && (noteSwitchOpener(opener), giveBack())} className="flex max-h-[76vh] w-full max-w-md flex-col overflow-hidden rounded-xl border border-hairline bg-surface shadow-card outline-none">
        <header className="flex items-start gap-3 border-b border-hairline px-5 py-4">
          <div className="min-w-0 flex-1">
            <h2 id="continue-dialog-title" className="text-section font-semibold text-ink">
              {title}
            </h2>
            {detail && <p className="mt-0.5 truncate text-meta text-ink-2">{detail}</p>}
          </div>
          <button type="button" onClick={onClose} aria-label="Close" title="Close (Esc)" data-testid="continue-dialog-close" className="-mr-1 rounded-lg px-2 py-1 text-section leading-none text-muted hover:bg-surface-2 hover:text-ink">
            ×
          </button>
        </header>
        <div className="flex min-h-0 flex-col gap-3 overflow-y-auto px-5 py-4">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
