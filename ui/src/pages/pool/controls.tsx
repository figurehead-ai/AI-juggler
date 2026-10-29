/**
 * The pool page's small controls, built on the row menus' pattern until
 * the shared controls (Picker, Checkbox, ChoiceLine; branch controls-0.3)
 * land — each is one component here, so swapping it is one change:
 *
 * - {@link Picker}: a button that says the choice, opening a menu of the
 *   options, each with a line of what it means.
 * - {@link Toggle}: an on/off switch with its label and a line of why.
 * - {@link Stepper}: a number with − and +, within its bounds.
 */
import { useId, useState, type JSX, type ReactNode } from "react";

import { MenuItem, RowMenu, menuPoint } from "../../components/RowMenu";

export type Choice = { id: string; label: string; words?: string };

export function Picker({
  label,
  heading,
  value,
  choices,
  disabled,
  onPick,
  testId,
  icon,
  variant = "field",
}: {
  /** What it picks, for screen readers ("What agents may do"). */
  label: string;
  heading: string;
  value: string;
  choices: Choice[];
  disabled?: boolean;
  onPick: (id: string) => void;
  testId?: string;
  icon?: ReactNode;
  /** `field`: a full-width line in a panel; `chip`: a header button. */
  variant?: "field" | "chip";
}): JSX.Element {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const current = choices.find((choice) => choice.id === value);
  return (
    <>
      <button
        type="button"
        disabled={disabled}
        aria-haspopup="menu"
        aria-expanded={at !== null}
        aria-label={`${label}: ${current?.label ?? value}`}
        onClick={(event) => setAt(menuPoint(event))}
        className={
          "flex items-center gap-2 rounded-lg border border-hairline text-left outline-none transition-colors hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-default disabled:opacity-60 " +
          (variant === "chip" ? "h-8 px-2.5 text-meta text-ink" : "w-full px-2.5 py-1.5 text-body text-ink")
        }
        data-testid={testId}
      >
        {icon}
        <span className="min-w-0 flex-1 truncate">{current?.label ?? value}</span>
        <svg aria-hidden="true" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" className="shrink-0 text-faint">
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>
      {at && (
        <RowMenu heading={heading} ariaLabel={label} at={at} onClose={() => setAt(null)} wide>
          {choices.map((choice) => (
            <MenuItem
              key={choice.id}
              checked={choice.id === value}
              onClick={() => {
                setAt(null);
                if (choice.id !== value) onPick(choice.id);
              }}
              testId={testId ? `${testId}-${choice.id}` : undefined}
            >
              <span className="flex items-start gap-2">
                <span aria-hidden="true" className={"mt-1.5 size-1.5 shrink-0 rounded-full " + (choice.id === value ? "bg-accent" : "bg-transparent")} />
                <span className="flex flex-col">
                  <span className="font-medium">{choice.label}</span>
                  {choice.words && <span className="text-label text-muted">{choice.words}</span>}
                </span>
              </span>
            </MenuItem>
          ))}
        </RowMenu>
      )}
    </>
  );
}

export function Toggle({ label, hint, on, disabled, onChange, testId }: { label: string; hint?: string; on: boolean; disabled?: boolean; onChange: (on: boolean) => void; testId?: string }): JSX.Element {
  const id = useId();
  return (
    <div className={"flex items-start gap-3" + (disabled ? " opacity-60" : "")}>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span id={`${id}-label`} className="text-body text-ink">
          {label}
        </span>
        {hint && (
          <span id={`${id}-hint`} className="text-label text-muted">
            {hint}
          </span>
        )}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={on}
        aria-labelledby={`${id}-label`}
        aria-describedby={hint ? `${id}-hint` : undefined}
        disabled={disabled}
        onClick={() => onChange(!on)}
        className={"relative mt-0.5 h-5 w-9 shrink-0 rounded-full border outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60 " + (on ? "border-accent bg-accent" : "border-hairline bg-surface-3")}
        data-testid={testId}
      >
        <span aria-hidden="true" className={"absolute top-0.5 size-3.5 rounded-full bg-white shadow transition-[left] " + (on ? "left-[18px]" : "left-0.5")} />
      </button>
    </div>
  );
}

export function Stepper({ label, value, min, max, disabled, onChange, testId, unit }: { label: string; value: number; min: number; max: number; disabled?: boolean; onChange: (value: number) => void; testId?: string; unit?: string }): JSX.Element {
  const button = "flex size-7 items-center justify-center rounded-md text-body text-ink outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-default disabled:text-faint disabled:hover:bg-transparent";
  return (
    <span className="inline-flex items-center gap-0.5 rounded-lg border border-hairline p-0.5" role="group" aria-label={label} data-testid={testId}>
      <button type="button" className={button} disabled={disabled || value <= min} onClick={() => onChange(value - 1)} aria-label={`Fewer: ${value - 1}`}>
        −
      </button>
      <span className="min-w-[2.5rem] text-center font-mono text-meta tabular-nums text-ink" aria-live="polite">
        {value}
        {unit ? ` ${unit}` : ""}
      </span>
      <button type="button" className={button} disabled={disabled || value >= max} onClick={() => onChange(value + 1)} aria-label={`More: ${value + 1}`}>
        +
      </button>
    </span>
  );
}
