/**
 * The switch dialog's rule for every choice that matters (the
 * maintainer, 2026-10-27): its consequence in a line, an example, an
 * inline "What does this mean?" and a link to the guide. This is the
 * inline part: a few short tips, and "Learn more" opening the guide's
 * section in the browser (the app picks the address).
 */
import type { JSX } from "react";
import { trustOpen } from "../../lib/ipc/trust";
import { Disclosure } from "../controls";

export function Explain({ tips, guide, testId }: { tips: string[]; guide: string; testId?: string }): JSX.Element {
  return (
    // The app's disclosure, a button an accessibility press opens (the
    // 0.4.6 rc2 walk P9), in a link's look.
    <Disclosure tone="link" className="rounded-lg text-meta text-ink" summary="What does this mean?" testId={testId ?? "switch-explain"}>
      <ul className="mt-1.5 flex list-disc flex-col gap-1 pl-5 leading-relaxed">
        {tips.map((tip) => (
          <li key={tip}>{tip}</li>
        ))}
      </ul>
      <button
        type="button"
        onClick={() => void trustOpen({ kind: "guide", id: guide })}
        className="mt-1.5 inline-flex min-h-6 items-center rounded text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        data-testid="switch-learn-more"
      >
        Learn more in the guide ↗
      </button>
    </Disclosure>
  );
}
