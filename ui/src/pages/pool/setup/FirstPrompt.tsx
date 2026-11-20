/**
 * Round 6 (Sam's walk): the ready line says what to do in the first
 * session, with Copy: 'Then tell Claude Code · Work: "Plan the work in
 * web, hand out tasks, and ask Codex to review."', built from the roles
 * (who leads, who reviews). Review and the diagram's ready box show it.
 */
import { useState, type JSX } from "react";

import { firstPrompt, type PoolShape } from "../../../lib/pool-sentences";

/** The first thing to tell the lead (null without one), and its Copy:
 * `copied` is null until it's tried, then whether it worked. Here and in
 * Review's LeadPrompt. */
export function useFirstPrompt(shape: Pick<PoolShape, "members" | "leadNext">, where: string): { prompt: ReturnType<typeof firstPrompt>; copied: boolean | null; copy: () => Promise<void> } {
  const [copied, setCopied] = useState<boolean | null>(null);
  const prompt = firstPrompt(shape, where);
  const copy = async () => {
    if (!prompt) return;
    try {
      await navigator.clipboard.writeText(prompt.words);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };
  return { prompt, copied, copy };
}

/** What Copy did, said under the words. */
export function CopiedNote({ copied, testId }: { copied: boolean | null; testId: string }): JSX.Element | null {
  if (copied === null) return null;
  return (
    <output className="block text-label text-muted" data-testid={`${testId}-copied`}>
      {copied ? "Copied: paste it into its first message." : "Couldn't copy it; select the words and copy them."}
    </output>
  );
}

export function FirstPrompt({ shape, where, testId = "first-prompt" }: { shape: Pick<PoolShape, "members" | "leadNext">; where: string; testId?: string }): JSX.Element | null {
  const { prompt, copied, copy } = useFirstPrompt(shape, where);
  if (!prompt) return null;
  return (
    <div className="flex flex-col gap-1" data-testid={testId}>
      <p className="text-meta text-ink">
        Then tell {prompt.to}: <q className="text-ink" data-testid={`${testId}-words`}>{prompt.words}</q>{" "}
        <button type="button" onClick={() => void copy()} className="rounded px-1 font-medium text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" aria-label={`Copy what to tell ${prompt.to}`} data-testid={`${testId}-copy`}>
          Copy
        </button>
      </p>
      <CopiedNote copied={copied} testId={testId} />
    </div>
  );
}
