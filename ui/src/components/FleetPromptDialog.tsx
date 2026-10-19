import { useEffect, useRef, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { fleetPromptDecide, fleetPrompts, linkStatus, type FleetPromptInfo } from "../lib/bridge";

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
    void check();
    const timer = window.setInterval(() => void check(), 5000);
    return () => {
      alive = false;
      window.clearInterval(timer);
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
          setPrompts(next);
          setNow(Date.now());
        }
      } catch {
        // Try again on the next tick.
      }
    };
    void load();
    const timer = window.setInterval(() => void load(), 1000);
    return () => {
      alive = false;
      window.clearInterval(timer);
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
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-6">
      <Card
        className="w-full max-w-sm p-5"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="fleet-prompt-title"
        aria-describedby="fleet-prompt-body"
        data-testid="fleet-prompt"
      >
        <p id="fleet-prompt-title" className="text-sm font-semibold">
          {prompt.peer_label} asks to act on this Mac
        </p>
        <p id="fleet-prompt-body" className="mt-1.5 text-xs leading-relaxed text-muted">
          It wants to <span className="text-ink">{prompt.action}</span>. Nothing happens unless you
          allow it. If you don't answer, it's denied in {left} s.
        </p>
        <p className="mt-2 text-[11px] text-muted">
          You chose "Ask each time" for {prompt.peer_label}; change it in Fleet.
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <Button ref={denyRef} size="sm" variant="secondary" onClick={() => void decide(false)} data-testid="fleet-prompt-deny">
            Deny
          </Button>
          <Button size="sm" variant="danger" onClick={() => void decide(true)} data-testid="fleet-prompt-allow">
            Allow
          </Button>
        </div>
      </Card>
    </div>
  );
}
