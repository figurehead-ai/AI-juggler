/**
 * AJ-E3011 round 2 (#3), round 3: "Start one now" for an App whose next
 * session leads — one button wherever it's offered (the lead control, the
 * App's card on the diagram, its inspector). It only opens New session on
 * that App in the pool's project; the user starts it.
 */
import type { JSX } from "react";

import { Button } from "../../../components/primitives";

export function StartLeadButton({ member, name, where, onStart, disabled = false, testId, className = "" }: { member: string; name: string; where: string; onStart: (member: string) => void; disabled?: boolean; testId: string; className?: string }): JSX.Element {
  return (
    <Button size="sm" variant="accent" disabled={disabled} className={className} onClick={() => onStart(member)} title={`New session on ${name} in ${where}: it leads once you start it`} data-testid={testId}>
      Start one now
    </Button>
  );
}
