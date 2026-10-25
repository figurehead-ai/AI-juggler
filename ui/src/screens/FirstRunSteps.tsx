/**
 * First run's and Add account's step marks, and the tools' terms line
 * (AJ-E1518): moved out of FirstRun.tsx (its size budget).
 */
import type { JSX } from "react";
import { ToolMark } from "../components/ToolMark";
import { toolSpec } from "../lib/tools";
import { trustOpen } from "../lib/ipc/trust";

export type Step = "found" | "fresh" | "folder" | "signin" | "access";

export function StepMarks({ step, skipAccess }: { step: Step; skipAccess: boolean }): JSX.Element {
  // The fresh-account and folder steps are the first step's other doors.
  const first = step === "fresh" ? "Name the account" : step === "folder" ? "Check the folder" : "Bring in your accounts";
  const steps: Array<{ id: Step; label: string }> = [
    { id: step === "fresh" || step === "folder" ? step : "found", label: first },
    { id: "signin", label: "Sign in where needed" },
    ...(skipAccess ? [] : [{ id: "access" as Step, label: "Computer access" }]),
  ];
  const at = steps.findIndex((entry) => entry.id === step);
  return (
    <ol className="flex flex-wrap items-center gap-3 text-xs text-muted" aria-label="First-run steps">
      {steps.map((entry, index) => (
        <li key={entry.id} className="flex items-center gap-3" aria-current={index === at ? "step" : undefined}>
          {index > 0 && <span aria-hidden="true" className="h-px w-6 bg-hairline" />}
          <span className={"flex items-center gap-2 " + (index === at ? "font-semibold text-ink" : "")}>
            <span
              aria-hidden="true"
              className={
                "flex size-5 items-center justify-center rounded-full text-label " +
                (index < at
                  ? "bg-success-soft text-success"
                  : index === at
                    ? "bg-accent text-canvas"
                    : "border border-hairline")
              }
            >
              {index < at ? "✓" : index + 1}
            </span>
            {entry.label}
          </span>
        </li>
      ))}
    </ol>
  );
}

/** AJ-E1518: what each tool's terms say about several accounts, with the
 * terms linked, while accounts are being added. The words are the
 * adapter's (`ToolSpec::terms`), checked against the terms on the day
 * they carry. */
export function TermsLine({ tools }: { tools: string[] }): JSX.Element | null {
  const rows = tools.flatMap((id) => {
    const terms = toolSpec(id).terms;
    return terms ? [{ id, terms }] : [];
  });
  if (rows.length === 0) return null;
  return (
    <ul className="flex flex-col gap-1.5 border-t border-hairline pt-4 text-xs text-muted" aria-label="Your accounts and the tools' terms" data-testid="terms-line">
      {rows.map(({ id, terms }) => (
        <li key={id} className="flex items-start gap-2">
          <ToolMark tool={id} size={14} className="mt-0.5 shrink-0 text-muted" />
          <span>
            {terms.accounts}{" "}
            <button type="button" onClick={() => void trustOpen({ kind: "terms", id })} className="text-accent underline underline-offset-2">
              {terms.name} ↗
            </button>
          </span>
        </li>
      ))}
    </ul>
  );
}
