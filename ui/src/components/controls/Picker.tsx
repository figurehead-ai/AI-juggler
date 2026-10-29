/**
 * 0.3 — the one Picker, in place of the system select (about 21 of
 * them, two wrappers): a button showing the value and a chevron, as
 * tall and round as the buttons beside it, that opens the row menu's
 * pattern (RowMenu) with a ✓ on the chosen line. Each option can say
 * what it does (a hint) and, greyed, why it can't be chosen (a reason).
 * Keyboard: Enter, Space, ↓ or ↑ open it on the chosen line; the arrows
 * move; Enter picks; Escape closes it (only it, never the sheet it is
 * in), and focus comes back to the button.
 */
import { useId, useRef, useState, type JSX, type KeyboardEvent, type ReactNode } from "react";
import { MenuItem, RowMenu } from "../RowMenu";
import { IconChevronDown } from "../icons-controls";
import { ChoiceLine, type ControlIcon } from "./ChoiceLine";

export interface PickerOption<T extends string = string> {
  value: T;
  label: string;
  /** What it does, under it in the menu. */
  hint?: string;
  icon?: ControlIcon;
  disabled?: boolean;
  /** Why it is greyed (shown under it). */
  reason?: string;
}

const SIZES = {
  sm: "h-7 gap-1.5 rounded-lg px-2.5 text-xs",
  md: "h-9 gap-2 rounded-xl px-3 text-sm",
} as const;

const LOOKS = {
  /** A field: the secondary button's surface and border. */
  field: "border border-hairline bg-surface-2 text-ink hover:border-muted/30 hover:bg-surface-3",
  /** Inline in a line of words or a toolbar: no box until hovered. */
  quiet: "text-ink hover:bg-surface-2",
} as const;

export function Picker<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
  heading,
  prefix,
  placeholder = "Choose…",
  size = "md",
  look = "field",
  disabled = false,
  reason,
  className = "",
  testId,
  id,
  describedBy,
}: {
  value: T | null | undefined;
  options: readonly PickerOption<T>[];
  onChange: (value: T) => void;
  /** What it picks ("Sort by"); with the value, the button's name. */
  ariaLabel?: string;
  /** A sentence-case line on top of the menu. */
  heading?: string;
  /** Words before the value, inside the button ("Tool:"). */
  prefix?: ReactNode;
  placeholder?: string;
  size?: keyof typeof SIZES;
  look?: keyof typeof LOOKS;
  disabled?: boolean;
  /** Why it is disabled (its tooltip, and said with it). */
  reason?: string;
  className?: string;
  testId?: string;
  id?: string;
  describedBy?: string;
}): JSX.Element {
  const button = useRef<HTMLButtonElement | null>(null);
  const [at, setAt] = useState<{ x: number; y: number; width: number } | null>(null);
  // A press on the button while it's open closes it (the menu's own
  // outside-press already did): the click that follows doesn't reopen.
  const closing = useRef(false);
  const base = useId().replace(/:/g, "");
  const chosen = options.find((option) => option.value === value);
  const shown = chosen?.label ?? placeholder;
  const withIcons = options.some((option) => option.icon !== undefined);
  const open = () => {
    const box = button.current?.getBoundingClientRect();
    if (!box) return;
    button.current?.focus();
    setAt({ x: box.left, y: box.bottom + 4, width: box.width });
  };
  const onKey = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      open();
    }
  };
  const why = disabled && reason ? `${base}-why` : undefined;
  const Icon = chosen?.icon;
  return (
    <>
      <button
        ref={button}
        id={id}
        type="button"
        aria-haspopup="menu"
        aria-expanded={at !== null}
        aria-label={ariaLabel ? `${ariaLabel}: ${shown}` : undefined}
        aria-describedby={[describedBy, why].filter(Boolean).join(" ") || undefined}
        title={why ? reason : undefined}
        data-testid={testId}
        data-value={value ?? ""}
        disabled={disabled}
        onMouseDown={() => {
          closing.current = at !== null;
        }}
        onClick={() => {
          if (closing.current) closing.current = false;
          else if (at) setAt(null);
          else open();
        }}
        onKeyDown={onKey}
        className={
          `inline-flex min-w-0 max-w-full items-center font-medium outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-default disabled:opacity-50 ${SIZES[size]} ${LOOKS[look]} ` +
          className
        }
      >
        {Icon && <Icon aria-hidden="true" width={14} height={14} className="shrink-0 text-muted" />}
        {prefix && <span className="shrink-0 font-normal text-muted">{prefix}</span>}
        <span className={"min-w-0 flex-1 truncate text-left " + (chosen ? "" : "text-muted")}>{shown}</span>
        <IconChevronDown aria-hidden="true" width={14} height={14} strokeWidth={2} className="shrink-0 text-muted" />
      </button>
      {why && (
        <span id={why} className="sr-only">
          {reason}
        </span>
      )}
      {at && (
        <RowMenu heading={heading ?? ""} ariaLabel={heading ?? ariaLabel ?? "Choices"} at={at} minWidth={at.width} onClose={() => setAt(null)}>
          {options.map((option, index) => (
            <MenuItem
              key={option.value}
              checked={option.value === value}
              disabled={option.disabled}
              describedBy={option.disabled && option.reason ? `${base}-r${index}` : undefined}
              testId={testId ? `${testId}-${option.value || "none"}` : undefined}
              onClick={() => {
                setAt(null);
                if (option.value !== value) onChange(option.value);
              }}
            >
              <ChoiceLine
                icon={withIcons ? (option.icon ?? null) : undefined}
                label={option.label}
                hint={option.hint}
                reason={option.disabled ? option.reason : undefined}
                reasonId={`${base}-r${index}`}
                checked={option.value === value}
                strong={option.value === value}
              />
            </MenuItem>
          ))}
        </RowMenu>
      )}
    </>
  );
}
