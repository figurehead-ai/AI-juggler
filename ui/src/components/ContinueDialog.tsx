/**
 * 0.3.2 — "Continue on another account…" opens the session's ranked
 * accounts in a dialog, over whatever page is showing (the maintainer's
 * 0.3.0 walk: the banner's button only opened the session's page, and
 * the list it focused sat unnoticed in the side column). One dialog for
 * a session AI Juggler runs and one another app runs; its body is the
 * page's own switch list, moved here while the dialog is open, so the two
 * can't drift. Picking an account opens the switch screen, as from the
 * page; Esc, ×, Cancel or a click outside close it and change nothing.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { noteSwitchOpener } from "../lib/handover-focus";
import { useEscapeCloses } from "../lib/use-escape-closes";
import { Button } from "./primitives";
import { DialogFrame, DialogOverlay } from "./DialogFrame";

/** The pickable accounts in either switch list. */
const TARGETS = "[data-testid='continue-target']:not(:disabled), button[data-row]:not(:disabled)";
/** Any pick that opens a switch: an account here or on another Mac. */
const PICKS = `${TARGETS}, [data-testid='remote-switch-target']:not(:disabled)`;
/** The Suggested one, in either list. */
const SUGGESTED = "[data-testid='continue-target'][data-suggested='true']:not(:disabled), button[data-row][aria-pressed='true']:not(:disabled)";

/** Where the keyboard goes back to: what opened the dialog, else (a
 * menu's line, gone with its menu) the page's own Switch account… (D-143), else
 * the page. */
function back(opener: Element | null): HTMLElement {
  const usable = (node: Element | null): node is HTMLElement => node instanceof HTMLElement && node.isConnected && node !== document.body && !node.closest("[role='menu']");
  if (usable(opener)) return opener;
  const door = document.querySelector("[data-testid='switch-account']");
  if (usable(door)) return door;
  return document.querySelector<HTMLElement>("main") ?? document.body;
}

export function ContinueDialog({ title, detail, onClose: close, children }: { title: string; detail?: string; onClose: () => void; children: ReactNode }): JSX.Element {
  const ref = useRef<HTMLDivElement | null>(null);
  // WCAG 2.4.3: the control that opened the dialog gets the keyboard
  // back, whether it closes or an account is picked. A pick hands it back
  // first, so the switch it opens returns there on Cancel (its rows here
  // are gone by then: 0.3.2's keyboard walk ended on the page's body).
  const [opener] = useState(() => back(document.activeElement));
  const giveBack = () => {
    const to = opener.isConnected ? opener : back(null);
    to.focus();
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
  return (
    <DialogOverlay className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-4 pt-[12vh]" onMouseDown={(event) => event.target === event.currentTarget && onClose()} data-testid="continue-dialog-backdrop">
      <DialogFrame
        frameRef={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="continue-dialog-title"
        tabIndex={-1}
        data-testid="continue-dialog"
        onClickCapture={(event) => (event.target as Element).closest(PICKS) && (noteSwitchOpener(opener), giveBack())}
        size="S"
        // Held near the top (pt-[12vh]): no taller than the room below it.
        className="max-h-[76vh] rounded-xl border border-hairline bg-surface shadow-card outline-none"
        headerClassName="flex items-start gap-3 px-5 py-4"
        header={
          <>
            <div className="min-w-0 flex-1">
              <h2 id="continue-dialog-title" className="text-section font-semibold text-ink">
                {title}
              </h2>
              {detail && <p className="mt-0.5 truncate text-meta text-ink-2">{detail}</p>}
            </div>
            <button type="button" onClick={onClose} aria-label="Close" title="Close (Esc)" data-testid="continue-dialog-close" className="-mr-1 rounded-lg px-2 py-1 text-section leading-none text-muted hover:bg-surface-2 hover:text-ink">
              ×
            </button>
          </>
        }
        bodyClassName="flex flex-col gap-3 px-5 pt-1 pb-4"
        bodyTestId="continue-dialog-body"
        // AJ-E2311 item 4 (the 0.4.6 walk at 1280×800: no Cancel in
        // sight): the way out stays under the list, however long it is.
        footerClassName="flex justify-end px-5 py-3"
        footerTestId="continue-dialog-footer"
        footer={
          <Button variant="ghost" type="button" onClick={onClose} data-testid="continue-dialog-cancel" aria-keyshortcuts="Escape">
            Cancel
          </Button>
        }
      >
        {children}
      </DialogFrame>
    </DialogOverlay>
  );
}
