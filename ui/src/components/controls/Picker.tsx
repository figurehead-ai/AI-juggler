/**
 * 0.3 — the one Picker, in place of the system select (about 21 of
 * them, two wrappers): a button showing the value and a chevron, as
 * tall and round as the buttons beside it, that opens the row menu's
 * pattern (RowMenu) with a ✓ on the chosen line. Each option can say
 * what it does (a hint) and, greyed, why it can't be chosen (a reason).
 * Keyboard: Enter, Space, ↓ or ↑ open it on the chosen line; the arrows
 * move; Enter picks; Escape closes it (only it, never the sheet it is
 * in), and focus comes back to the button.
 *
 * 0.3.2, the pickers rule (docs/plan/QUALITY.md "Finding things"): the
 * call site may name groups to show first ("Open now", "Recent", "In this
 * project"), then All; options that would show nothing (`empty`) go last;
 * past 8 options a search field sits on top, the arrows going on from it
 * and Enter picking the first match; at most 60 lines are drawn, the rest
 * found by typing.
 */
import { useId, useRef, useState, type JSX, type KeyboardEvent, type ReactNode } from "react";
import { MenuItem, MenuSeparator, RowMenu, type MenuAt } from "../RowMenu";
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
  /** It would show nothing (an empty pool in a filter): listed last. */
  empty?: boolean;
}

/** A group shown before All: its words and the values in it, in order. */
export interface PickerGroup<T extends string = string> {
  label: string;
  values: readonly T[];
}

/** Past this many options the menu has a search field. */
export const SEARCH_FROM = 9;
/** At most this many lines are drawn; typing finds the rest. */
const DRAWN = 60;

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
  groups = [],
  searchLabel = "Search",
}: {
  value: T | null | undefined;
  options: readonly PickerOption<T>[];
  /** Shown before All: the most used and most related here. */
  groups?: readonly PickerGroup<T>[];
  /** The search field's name ("Search pools"). */
  searchLabel?: string;
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
  const [at, setAt] = useState<(MenuAt & { width: number }) | null>(null);
  const [query, setQuery] = useState("");
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
    setQuery("");
    setAt({ x: box.left, y: box.bottom + 4, above: box.top - 4, right: box.right, width: box.width });
  };
  const onKey = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      open();
    }
  };
  const pick = (option: PickerOption<T>) => {
    setAt(null);
    if (option.value !== value) onChange(option.value);
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
        <RowMenu
          heading={heading ?? ""}
          ariaLabel={heading ?? ariaLabel ?? "Choices"}
          at={at}
          minWidth={at.width}
          onClose={() => setAt(null)}
          focusFirst={options.length >= SEARCH_FROM ? "input[data-picker-search]" : undefined}
        >
          {options.length >= SEARCH_FROM && (
            <input
              data-picker-search
              type="search"
              value={query}
              aria-label={searchLabel}
              placeholder={`${searchLabel}…`}
              data-testid={testId ? `${testId}-search` : undefined}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key !== "Enter") return;
                event.preventDefault();
                const first = sections(options, groups, query).flatMap((section) => section.options).find((option) => !option.disabled);
                if (first) pick(first);
              }}
              className="mx-0.5 mb-1 h-8 rounded-lg border border-hairline bg-surface-2 px-2.5 text-body text-ink outline-none placeholder:text-muted focus-visible:ring-2 focus-visible:ring-accent/60"
            />
          )}
          {lines(sections(options, groups, query), (option, key, index) => (
            <MenuItem
              key={key}
              checked={option.value === value}
              disabled={option.disabled}
              describedBy={option.disabled && option.reason ? `${base}-r${index}` : undefined}
              testId={testId ? `${testId}-${option.value || "none"}` : undefined}
              onClick={() => pick(option)}
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

interface Section<T extends string> {
  label: string | null;
  options: PickerOption<T>[];
  /** Matches left undrawn (typing finds them). */
  more: number;
}

/** What the menu lists: the groups (not while searching), then All with
 * the empty ones last; a search keeps what matches its words. */
export function sections<T extends string>(options: readonly PickerOption<T>[], groups: readonly PickerGroup<T>[], query: string): Section<T>[] {
  const words = query.trim().toLowerCase();
  const matches = (option: PickerOption<T>) => words === "" || `${option.label} ${option.hint ?? ""}`.toLowerCase().includes(words);
  const byValue = new Map(options.map((option) => [option.value, option]));
  const out: Section<T>[] = [];
  if (words === "")
    for (const group of groups) {
      const shown = group.values.map((value) => byValue.get(value)).filter((option): option is PickerOption<T> => option !== undefined);
      if (shown.length > 0) out.push({ label: group.label, options: shown, more: 0 });
    }
  const all = [...options.filter((option) => !option.empty), ...options.filter((option) => option.empty)].filter(matches);
  out.push({ label: out.length > 0 ? "All" : null, options: all.slice(0, DRAWN), more: Math.max(0, all.length - DRAWN) });
  return out;
}

/** The sections as menu lines: a quiet heading for each group and All,
 * a separator between them, "Nothing matches", and "N more: type to find". */
function lines<T extends string>(list: Section<T>[], line: (option: PickerOption<T>, key: string, index: number) => JSX.Element): JSX.Element[] {
  const out: JSX.Element[] = [];
  let index = 0;
  list.forEach((section, at) => {
    if (at > 0) out.push(<MenuSeparator key={`sep-${at}`} />);
    if (section.label) out.push(<span key={`head-${at}`} className="px-2 pt-0.5 text-label font-semibold text-muted">{section.label}</span>);
    for (const option of section.options) out.push(line(option, `${at}-${option.value}`, index++));
    if (section.options.length === 0) out.push(<span key={`none-${at}`} className="px-2 py-1 text-meta text-muted">Nothing matches</span>);
    if (section.more > 0) out.push(<span key={`more-${at}`} className="px-2 py-1 text-label text-muted">{section.more} more: type to find them</span>);
  });
  return out;
}
