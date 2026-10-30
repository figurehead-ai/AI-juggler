/**
 * 0.3 — one choice line, promoted from the rail's order menu (D-47):
 * icon · label · hint stacked on the label's indent · ✓ or a control on
 * the right. The same line in a menu (inside a MenuItem), in a Picker's
 * menu, and as an option in a sheet or Settings (`ChoiceOption`, with
 * its Radio or Checkbox). One grid, so labels and hints line up whether
 * a line has a hint or not.
 */
import { useId, type JSX, type ReactNode, type SVGProps } from "react";
import { IconCheck } from "../icons-controls";
import { Checkbox, Radio, type ToggleProps } from "./Toggle";

export type ControlIcon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

export function ChoiceLine({
  icon: Icon,
  label,
  hint,
  reason,
  reasonId,
  labelId,
  hintId,
  checked,
  end,
  strong,
  hintInline = false,
}: {
  /** The line's icon; `null` keeps its column empty so the labels still
   * line up with lines that have one; leave it out for no column. */
  icon?: ControlIcon | null;
  label: ReactNode;
  /** One line on what it does, under the label. */
  hint?: ReactNode;
  /** The hint on the label's line, muted, after it (a menu where some
   * lines have one and some don't: every line stays one height, 0.3's
   * View). */
  hintInline?: boolean;
  /** Why it can't be chosen now (a greyed line says why). */
  reason?: ReactNode;
  reasonId?: string;
  /** Ids for a control that is named by the label and described by the
   * hint (ChoiceOption). */
  labelId?: string;
  hintId?: string;
  /** A choice: ✓ on the right when it is the chosen one (the column is
   * kept either way, so lines don't shift). */
  checked?: boolean;
  /** What goes on the right instead of the ✓ (a value, a control). */
  end?: ReactNode;
  /** The label in semibold (the chosen one of a menu's choices). */
  strong?: boolean;
}): JSX.Element {
  const columns = Icon === undefined ? "grid-cols-[minmax(0,1fr)_auto]" : "grid-cols-[16px_minmax(0,1fr)_auto]";
  return (
    <span className={`grid w-full items-start gap-x-2 text-left ${columns}`}>
      {Icon !== undefined && (
        <span aria-hidden="true" className="flex h-[1.45em] items-center text-muted">
          {Icon && <Icon width={15} height={15} className="shrink-0" />}
        </span>
      )}
      <span className={hintInline ? "flex min-w-0 items-baseline gap-2" : "flex min-w-0 flex-col"}>
        <span id={labelId} data-choice-label className={(strong ? "font-semibold " : "") + (hintInline ? "shrink-0" : "")}>
          {label}
        </span>
        {hint && (
          <span id={hintId} className={"text-label leading-snug text-muted " + (hintInline ? "min-w-0 truncate" : "")}>
            {hint}
          </span>
        )}
        {reason && (
          <span id={reasonId} className="text-label leading-snug text-muted">
            {reason}
          </span>
        )}
      </span>
      <span className="flex h-[1.45em] items-center justify-end">
        {end ?? (checked !== undefined && <IconCheck aria-hidden="true" width={14} height={14} strokeWidth={2.5} className={"text-accent-hi " + (checked ? "" : "invisible")} />)}
      </span>
    </span>
  );
}

/**
 * A choice in a sheet or in Settings: the whole line is the label of its
 * Radio or Checkbox (on the right). The input props pass through (name,
 * checked, onChange, disabled, data-testid…).
 */
export function ChoiceOption({
  type,
  icon,
  label,
  hint,
  reason,
  look = "line",
  className = "",
  ...input
}: ToggleProps & {
  type: "radio" | "checkbox";
  /** A line in a list, or a card in a grid of equal cards (Settings ›
   * General's theme, density and order). */
  look?: "line" | "card";
  icon?: ControlIcon | null;
  label: ReactNode;
  hint?: ReactNode;
}): JSX.Element {
  const base = useId().replace(/:/g, "");
  const shownReason = input.disabled ? reason : undefined;
  const describedBy = [input["aria-describedby"], hint ? `${base}-hint` : undefined, shownReason ? `${base}-why` : undefined].filter(Boolean).join(" ") || undefined;
  // Named by its label alone (a test or a screen reader finds "Label",
  // not "Label and the whole hint"); the hint and reason describe it.
  const named = { "aria-labelledby": input["aria-label"] ? undefined : `${base}-label`, "aria-describedby": describedBy };
  const control =
    type === "radio" ? (
      <Radio {...input} {...named} title={input.title ?? shownReason} className="-my-[3px] -mr-[3px]" />
    ) : (
      <Checkbox {...input} {...named} title={input.title ?? shownReason} className="-my-[3px] -mr-[3px]" />
    );
  return (
    <label
      className={
        "flex rounded-lg text-body transition-colors " +
        (look === "card"
          ? "border px-3 py-2 " + (input.checked ? "border-accent/60 bg-accent-soft " : "border-hairline ")
          : "px-2 py-1.5 ") +
        (input.disabled ? "text-muted" : look === "card" && input.checked ? "text-ink" : "text-ink hover:bg-surface-2") +
        (className ? ` ${className}` : "")
      }
    >
      <ChoiceLine
        icon={icon}
        label={label}
        hint={hint}
        reason={shownReason}
        labelId={`${base}-label`}
        hintId={`${base}-hint`}
        reasonId={`${base}-why`}
        end={control}
      />
    </label>
  );
}
