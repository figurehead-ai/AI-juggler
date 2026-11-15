/**
 * 0.4.6 (AJ-E3011 §7): the agent-tools consent where it applies — a
 * compact card in the setup (and in the canvas's checklist), not a wall of
 * text above the work. It says what turning it on gives these sessions,
 * its consequence with an example, that it is one switch for every pool
 * (Settings › Agent access), then Turn on and Not now; once "Not now", it
 * says what won't work until it's on, and still offers Turn on.
 */
import { useState, type JSX } from "react";

import { Button } from "../../../components/primitives";
import { errorText } from "../../../lib/bridge";
import { agentBridgeAnswer, setAgentBridge, useAgentBridge } from "../../../lib/ipc/agent-bridge";

export const TOOLS_WHAT = "Let AI Juggler give these sessions the pool's tools: messages, shared notes and tasks.";
export const TOOLS_EXAMPLE = "Example: Claude can see that Codex is reviewing and send it a note.";
export const TOOLS_SCOPE = "One switch for every pool, in Settings › Agent access. Sessions get the tools when they next start in AI Juggler.";
export const TOOLS_OFF = "Until it's on, these sessions can't see each other, message or share tasks: the pool only groups them for you.";

export function PoolToolsCard({ testId = "pool-tools-card" }: { testId?: string }): JSX.Element | null {
  const info = useAgentBridge();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (info === null || (info.asked && info.on)) return null;
  const answer = (on: boolean) => {
    setBusy(true);
    setError(null);
    void agentBridgeAnswer(on).then(
      (next) => {
        setAgentBridge(next);
        setBusy(false);
      },
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  const declined = info.asked && !info.on;
  return (
    <section aria-labelledby={`${testId}-title`} className="flex flex-col gap-1.5 rounded-xl bg-surface-2 px-3.5 py-3" data-testid={testId}>
      <h4 id={`${testId}-title`} className="text-body font-semibold text-ink">
        {TOOLS_WHAT}
      </h4>
      <p className="text-meta text-ink-2">{TOOLS_EXAMPLE}</p>
      <p className="text-meta text-muted">{TOOLS_SCOPE}</p>
      {declined && (
        <p className="text-meta text-ink-2" data-testid={`${testId}-off`}>
          {TOOLS_OFF}
        </p>
      )}
      <span className="mt-1 flex gap-2">
        <Button size="sm" variant="accent" disabled={busy || !info.available} onClick={() => answer(true)} data-testid={`${testId}-on`}>
          Turn on
        </Button>
        {!declined && (
          <Button size="sm" variant="ghost" disabled={busy} onClick={() => answer(false)} data-testid={`${testId}-not-now`}>
            Not now
          </Button>
        )}
      </span>
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
