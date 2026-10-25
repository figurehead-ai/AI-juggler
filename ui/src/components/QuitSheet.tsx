/**
 * AJ-E1503 Req.3 — quitting with live sessions says how many are running
 * and what quitting does to them, before anything happens. ⌘Q and the
 * window's close button ask the backend; with nothing live it quits at
 * once and this sheet never shows.
 *
 * A terminal session ends when AI Juggler quits (its terminal lives in
 * this app); a desktop session keeps running on its own. Quit anyway,
 * Stop them first (a clean stop, then quit) or Cancel — nothing is
 * stopped without the click. A stop still held in its undo window is sent
 * before quitting either way. The sheet acknowledges the request when it
 * shows (else Rust quits after 3 s), Cancel tells Rust, and a second ⌘Q
 * while it is up quits anyway (quit.rs).
 */
import { useEffect, useState, type JSX } from "react";
import { Modal } from "./form";
import { Button } from "./primitives";
import { flushHeld } from "./Toast";
import { appQuit, appQuitAck, appQuitCancel, errorText, onQuitRequested, type QuitPlanInfo } from "../lib/bridge";

/** "1 session is running." / "3 sessions are running." */
export function runningWords(plan: QuitPlanInfo): string {
  return plan.live === 1 ? "1 session is running." : `${plan.live} sessions are running.`;
}

/** What quitting does to them, in words. AJ-E2310: a session AI Juggler
 * runs without a terminal ends with it and comes back when it opens again. */
export function quitEffectWords(plan: QuitPlanInfo): string {
  const one = plan.live === 1;
  const back = plan.come_back ?? 0;
  const only = (n: number) => n === plan.live;
  if (only(plan.end_on_quit)) return one ? "It ends when AI Juggler quits." : "They end when AI Juggler quits.";
  if (only(plan.keep_running)) return one ? "It keeps running after AI Juggler quits." : "They keep running after AI Juggler quits.";
  if (only(back)) return one ? "It comes back when AI Juggler opens again." : "They come back when AI Juggler opens again.";
  const parts: string[] = [];
  if (plan.end_on_quit > 0) parts.push(`${plan.end_on_quit === 1 ? "1 ends" : `${plan.end_on_quit} end`} when AI Juggler quits`);
  if (plan.keep_running > 0) parts.push(plan.keep_running === 1 ? "1 keeps running" : `${plan.keep_running} keep running`);
  if (back > 0) parts.push(back === 1 ? "1 comes back when it opens again" : `${back} come back when it opens again`);
  return `${parts.join("; ")}.`;
}

export function QuitSheet(): JSX.Element | null {
  const [plan, setPlan] = useState<QuitPlanInfo | null>(null);
  const [busy, setBusy] = useState<"quit" | "stop" | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(
    () =>
      onQuitRequested((next) => {
        setError(null);
        setBusy(null);
        setPlan(next);
      }),
    [],
  );
  // The sheet is up: tell Rust, or it quits after 3 s (a hung window must
  // never leave ⌘Q doing nothing).
  useEffect(() => {
    if (plan) void appQuitAck().catch(() => null);
  }, [plan]);
  if (!plan) return null;
  const one = plan.live === 1;
  const cancel = () => {
    if (busy !== null) return;
    setPlan(null);
    void appQuitCancel().catch(() => null);
  };
  const quit = async (stopFirst: boolean) => {
    setBusy(stopFirst ? "stop" : "quit");
    setError(null);
    try {
      await flushHeld();
      await appQuit(stopFirst);
    } catch (cause) {
      setBusy(null);
      setError(errorText(cause));
    }
  };
  return (
    <Modal title="Quit AI Juggler?" onClose={cancel} testId="quit-sheet">
      <div className="flex flex-col gap-4 text-xs">
        <p className="leading-relaxed">
          <span className="text-ink" data-testid="quit-running">
            {runningWords(plan)}
          </span>{" "}
          <span className="text-muted" data-testid="quit-effect">
            {quitEffectWords(plan)}
          </span>
        </p>
        {busy === "stop" && (
          <p className="text-muted" role="status">
            {one ? "Stopping it, then quitting…" : `Stopping ${plan.live} sessions, then quitting…`}
          </p>
        )}
        {error && <p className="text-warning">{error}</p>}
        <div className="flex flex-wrap justify-end gap-2">
          <Button size="sm" variant="ghost" disabled={busy !== null} onClick={cancel}>
            Cancel
          </Button>
          <Button size="sm" variant="secondary" disabled={busy !== null} onClick={() => void quit(true)} data-testid="quit-stop-first">
            {one ? "Stop it first" : "Stop them first"}
          </Button>
          <Button size="sm" variant="primary" disabled={busy !== null} onClick={() => void quit(false)} data-autofocus data-testid="quit-anyway">
            Quit anyway
          </Button>
        </div>
      </div>
    </Modal>
  );
}
