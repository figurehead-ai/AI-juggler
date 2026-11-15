/**
 * 0.4.6 (AJ-E3011 §3): the lead's placeholder, selected — "Lead: Claude
 * Code · Work's next session". It says what happens when that session
 * starts, and opens Who leads to change it.
 */
import type { JSX } from "react";

import { Button } from "../../components/primitives";
import { Section } from "./InspectPool";

export function InspectLeadNext({ app, onOpen }: { app: string | null; onOpen: () => void }): JSX.Element {
  return (
    <div className="flex flex-col" data-testid="inspect-lead-next">
      <Section title="What happens">
        <p className="text-meta text-ink-2">
          When {app ?? "the App"} starts its next session in this pool's projects, that session joins as the Lead: it plans the work and hands out tasks. Until then the pool works without a lead.
        </p>
        <p className="text-meta text-muted">One session leads (only one); the App's other sessions join as it says.</p>
        <Button size="sm" variant="ghost" className="self-start" onClick={onOpen} data-testid="inspect-lead-next-change">
          Change who leads
        </Button>
      </Section>
    </div>
  );
}
