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
 * before quitting either way.
 */
import { useEffect, useState, type JSX } from "react";
import { Modal } from "./form";
import { Button } from "./primitives";
import { flushHeld } from "./Toast";
import { appQuit, errorText, onQuitRequested, type QuitPlanInfo } from "../lib/bridge";

/** "1 session is running." / "3 sessions are running." */
export function runningWords(plan: QuitPlanInfo): string {
  return plan.live === 1 ? "1 session is running." : `${plan.live} sessions are running.`;
}

/** What quitting does to them, in words. */
export function quitEffectWords(plan: QuitPlanInfo): string {
  const one = plan.live === 1;
  if (plan.keep_running === 0) return one ? "It ends when AI Juggler quits." : "They end when AI Juggler quits.";
  if (plan.end_on_quit === 0) {
    return one ? "It keeps running after AI Juggler quits." : "They keep running after AI Juggler quits.";
  }
  const ends = plan.end_on_quit === 1 ? "1 ends" : `${plan.end_on_quit} end`;
  const keeps = plan.keep_running === 1 ? "1 keeps running" : `${plan.keep_running} keep running`;
  return `${ends} when AI Juggler quits; ${keeps}.`;
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
  if (!plan) return null;
  const one = plan.live === 1;
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
    <Modal title="Quit AI Juggler?" onClose={() => busy === null && setPlan(null)} testId="quit-sheet">
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
          <Button size="sm" variant="ghost" disabled={busy !== null} onClick={() => setPlan(null)}>
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
