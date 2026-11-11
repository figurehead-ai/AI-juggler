/**
 * D-71 (AJ-E3209) — Restart a running app or profile, after a confirm:
 * the dialog says what it stops ("2 sessions are working in it") before
 * anything happens, then the app is quit gracefully and started again on
 * the same profile (0.3.8's restart path). Every entry point (the rail's
 * App menu, the App's page, the Accounts screen's Apps) asks through
 * `askRestart`; one host shows the dialog.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "./primitives";
import { restartPlace } from "../lib/go-to";

const ASK = "aij:restart-app";

export type RestartAsk = {
  /** The remembered place to restart. */
  placeId: string;
  /** The App, whole ("Claude app · Work Max"). */
  name: string;
  /** Its sessions open now: what the restart closes (absent where the
   * door doesn't hold them: Settings › Desktop apps). */
  open?: number;
  /** Of those, the ones working now (a turn under way). */
  working?: number;
};

/** A restart's counts from an App's items: open, and working of those
 * (the go sitting: "1 session is working" for one that was idle). */
export function restartCounts(items: readonly { state: string; need: unknown }[]): { open: number; working: number } {
  return { open: items.length, working: items.filter((item) => item.state === "working" && item.need === null).length };
}

/** Asks to restart `ask`'s app: the host shows the confirm. */
export function askRestart(ask: RestartAsk): void {
  window.dispatchEvent(new CustomEvent<RestartAsk>(ASK, { detail: ask }));
}

/** What a restart stops, in words: "working" only for a session with a
 * turn under way, else "open". */
export function restartWords(open?: number, working = 0): string {
  if (open === undefined) return "Anything open in it closes; its conversations are kept and can be resumed.";
  if (open === 0) return "Nothing is open in it now.";
  const kept = `${open === 1 ? "its conversation is" : "their conversations are"} kept and can be resumed.`;
  if (working === 0) return `${open === 1 ? "1 session is" : `${open} sessions are`} open in it, none working. ${open === 1 ? "It closes" : "They close"}; ${kept}`;
  const rest = open - working;
  const also = rest > 0 ? ` (${rest} more open)` : "";
  return `${working === 1 ? "1 session is" : `${working} sessions are`} working in it${also}. ${working === 1 ? "Its turn stops" : "Their turns stop"} now; ${kept}`;
}

/** The one confirm, mounted once in the shell. */
export function RestartAppHost(): JSX.Element | null {
  const [ask, setAsk] = useState<RestartAsk | null>(null);
  useEffect(() => {
    const on = (event: Event) => setAsk((event as CustomEvent<RestartAsk>).detail);
    window.addEventListener(ASK, on);
    return () => window.removeEventListener(ASK, on);
  }, []);
  if (!ask) return null;
  return <RestartAppDialog ask={ask} onClose={() => setAsk(null)} />;
}

export function RestartAppDialog({ ask, onClose }: { ask: RestartAsk; onClose: () => void }): JSX.Element {
  const cancel = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    // The safe choice has the focus; Escape closes.
    cancel.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="restart-title"
        aria-describedby="restart-body"
        data-testid="restart-dialog"
        className="flex w-full max-w-md flex-col gap-3 rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        <h3 id="restart-title" className="text-body font-semibold text-ink">
          Restart {ask.name}?
        </h3>
        <p id="restart-body" className="text-meta text-ink-2" data-testid="restart-body">
          {restartWords(ask.open, ask.working)} AI Juggler quits it, waits for it to close, and starts it again on the same profile.
        </p>
        <div className="mt-1 flex justify-end gap-2">
          <Button ref={cancel} size="sm" variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button
            size="sm"
            variant="danger"
            data-testid="restart-confirm"
            onClick={() => {
              onClose();
              restartPlace(ask.placeId, true);
            }}
          >
            Restart
          </Button>
        </div>
      </div>
    </div>
  );
}
