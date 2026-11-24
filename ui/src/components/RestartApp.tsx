/**
 * D-71 (AJ-E3209) — Restart a running app or profile, after a confirm:
 * the dialog says what it stops ("2 sessions are working in it") before
 * anything happens, then the app is quit gracefully and started again on
 * the same profile (0.3.8's restart path). Every entry point (the rail's
 * App menu, the App's page, the Accounts screen's Apps) asks through
 * `askRestart`; one host shows the dialog.
 *
 * 0.5.1 F98: Quit…, beside Restart… wherever it is, asks through
 * `askQuit` in the same dialog: the app is quit the same way and left
 * closed (Keep alive waits for its next Open).
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "./primitives";
import { quitPlace, restartPlace } from "../lib/go-to";
import { DialogFrame, DialogOverlay } from "./DialogFrame";

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
  /** F98: quit it and leave it closed, not start it again. */
  quit?: boolean;
  /** F98: its Keep alive is on (the confirm says it won't start it). */
  keepAlive?: boolean;
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

/** F98: asks to quit `ask`'s app and leave it closed: the same host's
 * confirm, in Quit's words. */
export function askQuit(ask: Omit<RestartAsk, "quit">): void {
  askRestart({ ...ask, quit: true });
}

/** What happens after the confirm, in words: started again on the same
 * profile, or (F98) left closed, Keep alive too. */
export function afterWords(ask: Pick<RestartAsk, "quit" | "keepAlive">): string {
  if (!ask.quit) return "AI Juggler quits it, waits for it to close, and starts it again on the same profile.";
  return `AI Juggler quits it and waits for it to close. It stays closed until you start it again${ask.keepAlive ? ", Keep alive too" : ""}.`;
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
    <DialogOverlay className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <DialogFrame
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="restart-title"
        aria-describedby="restart-body"
        data-testid="restart-dialog"
        className="w-full max-w-md rounded-xl border border-hairline bg-surface shadow-card"
        headerClassName="px-5 pt-5"
        bodyClassName="flex flex-col gap-3 px-5 pt-3 pb-3"
        footerClassName="px-5 pt-3 pb-5"
        header={
          <h3 id="restart-title" className="text-body font-semibold text-ink">
            {ask.quit ? "Quit" : "Restart"} {ask.name}?
          </h3>
        }
        footer={
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
                if (ask.quit) quitPlace(ask.placeId, ask.name);
                else restartPlace(ask.placeId, true);
              }}
            >
              {ask.quit ? "Quit" : "Restart"}
            </Button>
          </div>
        }
      >
        <p id="restart-body" className="text-meta text-ink-2" data-testid="restart-body">
          {restartWords(ask.open, ask.working)} {afterWords(ask)}
        </p>
      </DialogFrame>
    </DialogOverlay>
  );
}
