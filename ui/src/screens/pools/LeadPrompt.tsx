/**
 * 0.4.6 (D-126): Review's side, under the team's drawing: the first
 * thing to tell its lead, built from the roles (round 6's `firstPrompt`,
 * the diagram's ready box says the same), with Copy, and where to paste
 * it. Nothing without a lead.
 */
import { useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { IconCopy } from "../../components/icons-actions";
import { firstPrompt, type PoolShape } from "../../lib/pool-sentences";

export function LeadPrompt({ shape, where, testId }: { shape: Pick<PoolShape, "members" | "leadNext">; where: string; testId: string }): JSX.Element | null {
  const [copied, setCopied] = useState<boolean | null>(null);
  const prompt = firstPrompt(shape, where);
  if (!prompt) return null;
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(prompt.words);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };
  return (
    <section aria-labelledby={`${testId}-title`} className="flex flex-col gap-2.5 px-4 pb-[18px] pt-4" data-testid={testId}>
      <span className="flex items-center gap-2">
        <h4 id={`${testId}-title`} className="section-label flex-1">
          First thing to tell your lead
        </h4>
        <Button size="sm" variant="secondary" onClick={() => void copy()} aria-label={`Copy what to tell ${prompt.to}`} data-testid={`${testId}-copy`}>
          <IconCopy aria-hidden="true" width={13} height={13} className="mr-1" />
          Copy
        </Button>
      </span>
      <q className="block rounded-xl bg-surface-2 px-3.5 py-3 text-meta text-ink-2" data-testid={`${testId}-words`}>
        {prompt.words}
      </q>
      <span className="text-label text-faint">Paste it into {prompt.to}'s first session in {where}.</span>
      {copied !== null && (
        <output className="block text-label text-muted" data-testid={`${testId}-copied`}>
          {copied ? "Copied: paste it into its first message." : "Couldn't copy it; select the words and copy them."}
        </output>
      )}
    </section>
  );
}
