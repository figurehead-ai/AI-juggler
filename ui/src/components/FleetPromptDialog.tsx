import { keepIfSame } from "../lib/same";
import { everyWhileShown } from "../lib/shown-window";
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "./primitives";
import { fleetPromptDecide, fleetPrompts } from "../lib/ipc/fleet-remote";
import { linkStatus } from "../lib/ipc/link";
import type { FleetPromptInfo } from "../types/generated/FleetPromptInfo";
import { DialogFrame, DialogOverlay } from "./DialogFrame";

/**
 * AJ-E709 — "Ask each time": a paired machine asked to act on THIS Mac.
 * The command waits (never runs) until this Mac's user clicks Allow; Deny
 * is the default focus, and silence is a Deny when the countdown ends.
 * Polls only while Link is on.
 */
export function FleetPromptDialog(): JSX.Element | null {
  const [prompts, setPrompts] = useState<FleetPromptInfo[]>([]);
  const [linkOn, setLinkOn] = useState(false);
  const [now, setNow] = useState(Date.now());
  const denyRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    let alive = true;
    const check = async () => {
      try {
        const status = await linkStatus();
        if (alive) setLinkOn(status.enabled);
      } catch {
        // Link state unavailable: nothing to ask.
      }
    };
    const stop = everyWhileShown(() => void check(), 5000, { now: true });
    return () => {
      alive = false;
      stop();
    };
  }, []);

  useEffect(() => {
    if (!linkOn) {
      setPrompts([]);
      return;
    }
    let alive = true;
    const load = async () => {
      try {
        const next = await fleetPrompts();
        if (alive) {
          setPrompts((was) => keepIfSame(was, next));
          // the countdown's clock, only while something counts down
          if (next.length > 0) setNow(Date.now());
        }
      } catch {
        // Try again on the next tick.
      }
    };
    const stop = everyWhileShown(() => void load(), 1000, { now: true });
    return () => {
      alive = false;
      stop();
    };
  }, [linkOn]);

  const prompt = prompts[0] ?? null;
  useEffect(() => {
    if (prompt) denyRef.current?.focus();
  }, [prompt?.id]);

  if (!prompt) return null;
  const left = Math.max(0, Math.round((prompt.expires_at_ms - now) / 1000));
  const decide = async (allow: boolean) => {
    await fleetPromptDecide(prompt.id, allow).catch(() => null);
    setPrompts((now) => now.filter((p) => p.id !== prompt.id));
  };
  return (
    <DialogOverlay className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4">
      <DialogFrame
        className="sheen rounded-card border border-hairline bg-surface shadow-card w-full max-w-sm"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="fleet-prompt-title"
        aria-describedby="fleet-prompt-body"
        data-testid="fleet-prompt"
        bodyClassName="px-5 pb-1.5"
        footerClassName="px-5 pt-2.5 pb-5"
        footer={
          <div className="flex justify-end gap-2">
            <Button ref={denyRef} size="sm" variant="secondary" onClick={() => void decide(false)} data-testid="fleet-prompt-deny">
              Deny
            </Button>
            <Button size="sm" variant="danger" onClick={() => void decide(true)} data-testid="fleet-prompt-allow">
              Allow
            </Button>
          </div>
        }
        headerClassName="px-5 pt-5"
        header={
          <p id="fleet-prompt-title" className="text-sm font-semibold">
            {prompt.peer_label} asks to act on this Mac
          </p>
        }
      >
        <p id="fleet-prompt-body" className="mt-1.5 text-xs leading-relaxed text-ink-2">
          It wants to <span className="text-ink">{prompt.action}</span>. Nothing happens unless you
          allow it. If you don't answer, it's denied in {left} s.
        </p>
        <p className="mt-2 text-label text-muted">
          You chose "Ask each time" for {prompt.peer_label}; change it in Fleet.
        </p>
      </DialogFrame>
    </DialogOverlay>
  );
}
