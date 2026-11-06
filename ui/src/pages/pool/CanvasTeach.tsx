/**
 * D-62 decision 18 (AJ-E3008 req. 1): the empty canvas teaches the order
 * (wireframe 5): three numbered steps, the next one to do the only
 * primary button; each opens the control it needs. "Use a template" is a
 * click away. Nothing here starts a session.
 */
import type { JSX } from "react";

import { Button } from "../../components/primitives";
import { MACHINES_WORD } from "../../lib/machine-words";

export function CanvasTeach({
  scope,
  scopeChosen,
  onScope,
  onAdd,
  onTemplate,
  inset,
}: {
  /** Where it works now, in words. */
  scope: string;
  /** The user has chosen where it works (step 1 is done). */
  scopeChosen: boolean;
  onScope: () => void;
  onAdd: () => void;
  onTemplate: () => void;
  /** Room left for the open palette. */
  inset: boolean;
}): JSX.Element {
  const steps = [
    { title: "Choose where it works", words: `Every project, or the ones you pick. Agents see only these. Now: ${scope}.`, action: scopeChosen ? "Change" : "Choose projects", onClick: onScope, done: scopeChosen },
    { title: "Add who's in it", words: `Sessions, or whole Apps, Accounts, Tools or ${MACHINES_WORD}, so new sessions join by themselves.`, action: "Add members", onClick: onAdd, done: false },
    { title: "Pick a lead and how they talk", words: "Click a member for its role, or a line between two for their talk.", action: "Later", onClick: null, done: false },
  ];
  const next = steps.findIndex((step) => !step.done);
  return (
    // Centred when there's room; scrolls (never clips) on a short canvas.
    <div className={"pointer-events-none absolute inset-0 flex overflow-y-auto p-4" + (inset ? " pl-[16.5rem]" : "")} data-testid="canvas-empty" data-canvas-scroll>
      <div className="pointer-events-auto m-auto flex w-full max-w-lg flex-col gap-3 rounded-2xl border border-hairline bg-surface p-4 shadow-card">
        <div>
          <h3 className="text-section text-ink">Three steps to a working pool</h3>
          <p className="text-meta text-ink-2">Each step opens the control it needs. Nothing starts a session.</p>
        </div>
        <ol className="flex flex-col gap-2">
          {steps.map((step, at) => (
            <li key={step.title} className="grid grid-cols-[24px_1fr] items-start gap-x-3 gap-y-2 rounded-xl bg-surface-2 px-3 py-2.5" data-testid="canvas-teach-step">
              <span className={"flex size-6 items-center justify-center rounded-full text-label font-semibold " + (step.done ? "bg-success text-white" : at === next ? "bg-accent text-white" : "bg-surface-3 text-muted")}>
                {step.done ? "✓" : at + 1}
              </span>
              <span className="flex min-w-0 flex-col">
                <span className="text-body font-semibold text-ink">{step.title}</span>
                <span className="text-meta text-ink-2">{step.words}</span>
              </span>
              <span className="col-start-2">
                {step.onClick ? (
                  <Button size="sm" variant={at === next ? "primary" : "secondary"} onClick={step.onClick} data-testid={at === 0 ? "canvas-teach-scope" : "canvas-teach-add"}>
                    {step.action}
                  </Button>
                ) : (
                  <span className="text-meta text-muted">{step.action}</span>
                )}
              </span>
            </li>
          ))}
        </ol>
        <p className="text-meta text-ink-2">
          Prefer a head start?{" "}
          <button type="button" onClick={onTemplate} className="font-medium text-accent-hi underline-offset-2 hover:underline" data-testid="canvas-teach-template">
            Use a template
          </button>
        </p>
      </div>
    </div>
  );
}
