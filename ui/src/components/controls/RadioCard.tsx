/**
 * 0.4.6 (D-126) — visual choice cards for the making screens: a logo or
 * icon, a title, one sub-line, and a check mark once chosen. The chosen
 * card takes the pool's chosen look (CHOSEN_CARD: the accent ring over a
 * raised surface), its keyboard focus a ring of its own further out.
 *
 * - `RadioCards` + `RadioCard`: one of a few (Who leads). A radio group
 *   of radios with one tab stop: the arrows move and choose (Home, End
 *   too), Space and Enter choose the focused one.
 * - `CheckCard`: any of a few (Who's in), a checkbox; Space and Enter
 *   flip it.
 *
 * `layout`: `row` (logo left, words beside: the kit's) or `stack` (logo
 * on top: a grid of apps). A card that can't be chosen now is faded and
 * says why. Each card is a label around its own native input (unseen),
 * so the control, its name and its keys are the platform's.
 *
 * 0.4.8 (the 0.4.7 nightly walk): a press on a template card's words
 * found nothing to press; only its radio took one, a pixel in a corner.
 * The input now covers the whole card, over its face, and the face is
 * shown, not read: the input is the card for screen readers, the pointer
 * and a press by place, its name the title and its description the rest
 * of the card's words. A part of the face that shows a hint on hover sits
 * above the input (positioned, after it).
 */
import { Children, createContext, useContext, useId, type JSX, type KeyboardEvent, type ReactNode } from "react";
import { IconCheck } from "../icons-controls";
import { CHOSEN_CARD } from "./selected";

type Layout = "row" | "stack";

interface CardWords {
  /** A ToolLogo (40 px on a choice), or an icon in a tile. */
  logo?: ReactNode;
  title: ReactNode;
  /** One line under the title. */
  sub?: ReactNode;
  /** More under the sub-line (a count, a tag). */
  children?: ReactNode;
  disabled?: boolean;
  /** Why it can't be chosen now. */
  reason?: string;
  layout?: Layout;
  testId?: string;
}

/** The check mark in the card's corner: a ring, filled with ✓ once chosen. */
function Mark({ on, shape }: { on: boolean; shape: "round" | "square" }): JSX.Element {
  return (
    <span
      aria-hidden="true"
      className={
        "absolute right-3.5 top-3.5 flex size-5 items-center justify-center " +
        (shape === "round" ? "rounded-full " : "rounded-md ") +
        (on ? "bg-accent-fill text-white" : "inset-ring-[1.5px] inset-ring-faint")
      }
    >
      {on && <IconCheck width={12} height={12} strokeWidth={3} />}
    </span>
  );
}

/** The card's body: logo, words, more. Its input is named by the title
 * and described by the rest; the words stay readable, because a label
 * hidden from assistive tech is no label (axe's label-title-only). */
function Body({ logo, title, sub, children, layout = "row", titleId, subId, moreId }: CardWords & { titleId: string; subId: string; moreId: string }): JSX.Element {
  return (
    <span className={layout === "row" ? "flex min-w-0 items-center gap-3.5 pr-7" : "flex min-w-0 flex-col items-start gap-2.5 pr-7"}>
      {logo && <span className="flex shrink-0">{logo}</span>}
      <span className="flex min-w-0 flex-col gap-0.5">
        <span id={titleId} className="text-body font-semibold text-ink">
          {title}
        </span>
        {sub && (
          <span id={subId} className="text-meta text-ink-2">
            {sub}
          </span>
        )}
        {hasMore(children) && (
          <span id={moreId} className="flex min-w-0 flex-col gap-0.5">
            {children}
          </span>
        )}
      </span>
    </span>
  );
}

/** Whether a card has more under its sub-line (a count, a tag). */
const hasMore = (children: ReactNode): boolean => Children.toArray(children).length > 0;

const CARD = "relative flex w-full min-w-0 rounded-card px-4 py-3.5 text-left transition-[background-color,box-shadow] ";
const AT_REST = "bg-surface inset-ring-1 inset-ring-hairline/75 hover:inset-ring-faint/60";
/** CARD_FOCUS, on the card while its (unseen) input has keyboard focus. */
const FOCUS_INSIDE = "has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-4 has-[:focus-visible]:outline-ink-2";

function cardLook(on: boolean, disabled: boolean): string {
  return CARD + (on ? CHOSEN_CARD : AT_REST) + " " + FOCUS_INSIDE + (disabled ? " opacity-60" : " cursor-pointer");
}

function useIds(words: CardWords): { titleId: string; subId: string; moreId: string; whyId: string; describedBy: string | undefined } {
  const base = useId().replace(/:/g, "");
  const ids = { titleId: `${base}-title`, subId: `${base}-sub`, moreId: `${base}-more`, whyId: `${base}-why` };
  const why = (words.disabled ?? false) && words.reason;
  const describedBy = [words.sub ? ids.subId : null, hasMore(words.children) ? ids.moreId : null, why ? ids.whyId : null].filter(Boolean).join(" ") || undefined;
  return { ...ids, describedBy };
}

/** What a card shows beside its input: the mark, the words, and why it
 * can't be chosen now (for screen readers; the card's title says it too). */
function CardFace({ on, shape, words, ids }: { on: boolean; shape: "round" | "square"; words: CardWords; ids: ReturnType<typeof useIds> }): JSX.Element {
  const disabled = words.disabled ?? false;
  return (
    <>
      <Mark on={on} shape={shape} />
      <Body {...words} titleId={ids.titleId} subId={ids.subId} moreId={ids.moreId} />
      {disabled && words.reason && (
        <span id={ids.whyId} aria-hidden="true" className="sr-only">
          {words.reason}
        </span>
      )}
    </>
  );
}

/** The card's own input: unseen, over the whole card, the control
 * screen readers, keys, the pointer and a press anywhere on it use. */
const UNSEEN = "peer absolute inset-0 m-0 size-full cursor-pointer appearance-none opacity-0 disabled:cursor-default";

const Group = createContext<{ name: string; value: string | null; onChange: (value: string) => void } | null>(null);

/** The arrows (and Home, End) move to the next card that can be chosen
 * and choose it; Enter chooses the focused one, as Space does. */
function onRadioKey(event: KeyboardEvent<HTMLInputElement>, choose: () => void): void {
  if (event.key === "Enter") {
    event.preventDefault();
    choose();
    return;
  }
  const keys = ["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft", "Home", "End"];
  const group = event.currentTarget.closest("[role='radiogroup']");
  if (!keys.includes(event.key) || !group) return;
  const radios = [...group.querySelectorAll<HTMLInputElement>("input[type='radio']:not(:disabled)")];
  const at = radios.indexOf(event.currentTarget);
  const step = event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
  const next = event.key === "Home" ? 0 : event.key === "End" ? radios.length - 1 : (Math.max(0, at) + step + radios.length) % radios.length;
  event.preventDefault();
  radios[next]!.focus();
  radios[next]!.click();
}

/** One of a few, as cards. */
export function RadioCards({
  label,
  value,
  onChange,
  children,
  columns = 1,
  testId,
}: {
  /** The group's name for screen readers ("Who leads"). */
  label: string;
  value: string | null;
  onChange: (value: string) => void;
  children: ReactNode;
  /** Side by side on a wide form. */
  columns?: 1 | 2 | 3;
  testId?: string;
}): JSX.Element {
  const name = useId();
  const grid = columns === 1 ? "grid-cols-1" : columns === 2 ? "grid-cols-1 sm:grid-cols-2" : "grid-cols-1 sm:grid-cols-3";
  return (
    <Group.Provider value={{ name, value, onChange }}>
      <div role="radiogroup" aria-label={label} className={`grid min-w-0 gap-2.5 ${grid}`} data-testid={testId}>
        {children}
      </div>
    </Group.Provider>
  );
}

/** One card of `RadioCards`. */
export function RadioCard({ value, ...words }: CardWords & { value: string }): JSX.Element {
  const group = useContext(Group);
  const on = group?.value === value;
  const disabled = words.disabled ?? false;
  const ids = useIds(words);
  const choose = () => !disabled && group?.onChange(value);
  return (
    <label className={cardLook(on, disabled)} title={disabled ? words.reason : undefined} data-testid={words.testId} data-value={value}>
      <input
        type="radio"
        name={group?.name}
        value={value}
        checked={on}
        disabled={disabled}
        aria-labelledby={ids.titleId}
        aria-describedby={ids.describedBy}
        onChange={choose}
        onKeyDown={(event) => onRadioKey(event, choose)}
        className={UNSEEN}
      />
      <CardFace on={on} shape="round" words={words} ids={ids} />
    </label>
  );
}

/** Any of a few, as cards: one card, ticked or not. Enter flips it, as
 * Space does. */
export function CheckCard({ checked, onChange, ...words }: CardWords & { checked: boolean; onChange: (next: boolean) => void }): JSX.Element {
  const disabled = words.disabled ?? false;
  const ids = useIds(words);
  return (
    <label className={cardLook(checked, disabled)} title={disabled ? words.reason : undefined} data-testid={words.testId}>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        aria-labelledby={ids.titleId}
        aria-describedby={ids.describedBy}
        onChange={(event) => onChange(event.target.checked)}
        onKeyDown={(event) => {
          if (event.key !== "Enter" || disabled) return;
          event.preventDefault();
          onChange(!checked);
        }}
        className={UNSEEN}
      />
      <CardFace on={checked} shape="square" words={words} ids={ids} />
    </label>
  );
}
