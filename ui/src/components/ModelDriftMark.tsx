/**
 * 0.5.1 F77 — a session whose model changed since it started, marked
 * beside its row: its own icon (a chip, never Switch account's ⇄), quiet
 * by tone (no outline, PAT-SURF-1; level 1, never amber, AJ-E3603 R2),
 * named in words in the drawn tip on hover AND keyboard focus
 * (`TipButton`, F61's pattern), never a browser `title`. The words stay
 * its accessible name; a click opens the session, whose Where it runs
 * says the change in full. Its hit box is 24 px (WCAG 2.5.8, axe
 * target-size: an 18 px button over its row read as too small); the
 * chip drawn inside stays 18 px, so the look is unchanged.
 */
import type { JSX } from "react";
import { IconModelChanged } from "./icons";
import { TipButton } from "./NameTip";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** The model it started with and the one it runs now, when they differ. */
export function driftOf(session: Pick<InstanceInfo, "launch_model" | "running_model">): { launched: string; running: string } | null {
  const { launch_model: launched, running_model: running } = session;
  return launched !== null && running !== null && launched !== running ? { launched, running } : null;
}

/** "Model changed: launched with opus, now running sonnet-5". */
export function driftWords({ launched, running }: { launched: string; running: string }): string {
  return `Model changed: launched with ${launched}, now running ${running}`;
}

export function ModelDriftMark({ drift, onOpen, className = "" }: { drift: { launched: string; running: string }; onOpen: () => void; className?: string }): JSX.Element {
  const words = driftWords(drift);
  return (
    <TipButton
      tip={words}
      aria-label={words}
      data-testid="model-drift"
      onClick={(event) => {
        event.stopPropagation();
        onOpen();
      }}
      className={"group/drift inline-flex size-6 shrink-0 items-center justify-center text-muted outline-none hover:text-ink-2 " + className}
    >
      <span className="inline-flex size-[18px] items-center justify-center rounded-md group-hover/drift:bg-surface-3 group-focus-visible/drift:ring-1 group-focus-visible/drift:ring-accent/60">
        <IconModelChanged aria-hidden="true" width={12} height={12} />
      </span>
    </TipButton>
  );
}
