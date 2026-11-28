/**
 * AJ-E2110 — the parts every Settings pane is made of: the pane's title,
 * sections (a header, a rounded card, a one-line footer saying what it
 * does), rows (a title, a line of why, and a control on the right) and
 * the switch. One look for every pane.
 *
 * AJ-E1558 (D-126 rich mode, PAT-SURF, PAT-COLOR): a section's rows sit
 * on its card's tone, no outline, no rule between them, no shadow; every
 * row leads with its icon chip (the accent's soft step while it is on) or
 * its App's own logo; the switch is the kit's (the accent's fill while on).
 */
import { useEffect, useId, useRef, type JSX, type ReactNode } from "react";
import { InSettingsSection } from "../../components/CardTitle";
import { RowLead, type RowIcon } from "../../components/controls";

export type { RowIcon };
import { IconWarn } from "../../components/icons-menu";
import { matchedRow, splitAnchor } from "./settings-index";

/** What the user does that ends a pane's landing: from then on the
 * scroll is theirs. */
const USER_INPUT = ["wheel", "pointerdown", "keydown", "touchstart"] as const;

/** How long a pane waits for its anchored section to appear (a section
 * drawn from data read on open, as Accounts' Tools on this Mac is), and
 * keeps the landing in view while the sections above it fill in. */
const ANCHOR_WAIT_MS = 2_000;
/** How long a searched row stays tinted. */
const HIT_TINT_MS = 2_400;

/** Opens every fold between `section` and `row`, so a searched setting
 * folded away (Notifications' quiet hours) is shown. */
function openFolds(row: HTMLElement, section: HTMLElement): void {
  for (let at: HTMLElement | null = row; at && at !== section; at = at.parentElement) {
    if (at === row) continue;
    // The app's Disclosure (a button, round 7), every fold's (the
    // native-controls lint keeps <details> out): pressed open.
    if (at.matches("[data-disclosure]:not([data-open])")) at.querySelector<HTMLElement>(":scope > button[aria-expanded]")?.click();
  }
}

/** A brief accent tint on the searched row, with rounded corners (design
 * review 34: not a square outline the pane's width). */
function tint(row: HTMLElement): void {
  row.setAttribute("data-settings-hit", "");
  window.setTimeout(() => row.removeAttribute("data-settings-hit"), HIT_TINT_MS);
  if (typeof row.animate !== "function") return;
  const own = getComputedStyle(row).backgroundColor;
  row.animate(
    [
      { backgroundColor: "var(--color-accent-soft)", borderRadius: "0.5rem" },
      { backgroundColor: "var(--color-accent-soft)", borderRadius: "0.5rem", offset: 0.6 },
      { backgroundColor: own, borderRadius: "0.5rem" },
    ],
    { duration: HIT_TINT_MS, easing: "ease-out" },
  );
}

/** A pane: its title, then its sections. Opens on `anchor`'s section
 * when a link asked for one; a search's anchor (`searchAnchor`) lands on
 * the row its words name, unfolds it and tints it (design review 25). */
export function Pane({
  id,
  title,
  anchor,
  head,
  children,
}: {
  id: string;
  title: string;
  anchor?: string;
  /** A head of its own in place of the title alone (a crumb, a lead
   * line, facts beside it); it holds the pane's h2. */
  head?: ReactNode;
  children: ReactNode;
}): JSX.Element {
  const pane = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!anchor) return;
    const { section, words } = splitAnchor(anchor);
    const find = () => document.getElementById(`settings-${section}`);
    let frame = 0;
    let landed = false;
    let lit: HTMLElement | null = null;
    const jump = () => {
      const at = find();
      if (!at) return;
      const row = words.length > 0 ? matchedRow(at, words) : null;
      if (row) openFolds(row, at);
      (row ?? at).scrollIntoView?.({ block: row ? "center" : "start" });
      if (!landed) document.getElementById(`settings-${section}-title`)?.focus({ preventScroll: true });
      landed = true;
      // The best row can change as the section fills in: tint that one.
      if (row && row !== lit) {
        lit = row;
        tint(row);
      }
    };
    const land = () => {
      jump();
      // Once more after the screen around it has set its own scroll.
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(jump);
    };
    if (find()) land();
    // A section drawn once its data is read isn't there yet: jump when it
    // appears, not on whichever frame the read happened to beat (the
    // visual gate caught the pane opening at its top on some runs). Once
    // there, cards above it that fill in later would push it away (search
    // "mute" opened Notifications with Muted off screen): land again on
    // each change, until the wait ends or the user scrolls, clicks or types.
    const root = pane.current;
    if (!root || typeof MutationObserver === "undefined") return () => cancelAnimationFrame(frame);
    const watch = new MutationObserver(() => {
      if (find()) land();
    });
    const stop = () => {
      watch.disconnect();
      for (const kind of USER_INPUT) window.removeEventListener(kind, stop, true);
    };
    watch.observe(root, { childList: true, subtree: true });
    for (const kind of USER_INPUT) window.addEventListener(kind, stop, true);
    const timer = window.setTimeout(stop, ANCHOR_WAIT_MS);
    return () => {
      stop();
      window.clearTimeout(timer);
      cancelAnimationFrame(frame);
    };
  }, [anchor]);
  return (
    <div ref={pane} className="flex flex-col gap-7" data-testid={`settings-pane-${id}`} data-pane={id}>
      {head ?? <h2 className="text-title text-ink">{title}</h2>}
      {children}
    </div>
  );
}

/** One section: a 15 px header (the readability scale), its card (rows, or a settings card
 * that brings its own surface), and a muted one-line footer. */
export function PaneSection({
  id,
  title,
  footer,
  card = false,
  children,
}: {
  id: string;
  title: string;
  footer?: ReactNode;
  /** The child is a settings card with its own surface (and a title the
   * header already says); otherwise the children are rows. */
  card?: boolean;
  children: ReactNode;
}): JSX.Element {
  return (
    <section
      id={`settings-${id}`}
      aria-labelledby={`settings-${id}-title`}
      className="flex scroll-mt-4 flex-col gap-2"
      data-settings-section={id}
    >
      {/* Fits its words, rounded: the focus ring a link or search lands
          with hugs the title, not the pane's width (design review 34). */}
      <h3 id={`settings-${id}-title`} tabIndex={-1} className="w-fit rounded-md px-1 text-section font-semibold text-ink">
        {title}
      </h3>
      <InSettingsSection.Provider value>
        {card ? (
          <div className="flex flex-col gap-3">{children}</div>
        ) : (
          <ul className="flex flex-col gap-0.5 rounded-card bg-surface p-1.5 inset-ring-1 inset-ring-frame">{children}</ul>
        )}
      </InSettingsSection.Provider>
      {footer && <p className="px-1 text-meta text-ink-2">{footer}</p>}
    </section>
  );
}

/** One row: its icon, what it is, one line of why, and its control. */
export function SettingRow({
  title,
  description,
  icon,
  on = false,
  control,
  testId,
  titleId,
  descriptionId,
  children,
}: {
  title: ReactNode;
  description?: ReactNode;
  /** Every row leads with one (D-126 rich mode, TGL-2). */
  icon: RowIcon;
  /** The row is on or chosen: its chip takes the accent's soft step. */
  on?: boolean;
  control?: ReactNode;
  testId?: string;
  titleId?: string;
  descriptionId?: string;
  /** Anything under the row's words (a note, a status line). */
  children?: ReactNode;
}): JSX.Element {
  return (
    <li className="flex items-center gap-3 rounded-[10px] px-3 py-[9px]" data-testid={testId}>
      <RowLead icon={icon} on={on} />
      <div className="flex min-w-0 flex-1 flex-col">
        <span id={titleId} className="text-body font-medium text-ink">
          {title}
        </span>
        {description && (
          <span id={descriptionId} className="text-meta text-muted">
            {description}
          </span>
        )}
        {children}
      </div>
      {control && <div className="flex shrink-0 items-center gap-2">{control}</div>}
    </li>
  );
}

/** A row with a switch, labelled by the row's title and described by
 * its line of why. */
export function SwitchRow({
  title,
  description,
  icon,
  checked,
  disabled = false,
  onChange,
  testId,
  note,
  noteTone = "info",
}: {
  title: string;
  description: ReactNode;
  icon: RowIcon;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  testId?: string;
  note?: string | null;
  /** How the note reads: a plain status line, or a warning. */
  noteTone?: RowNoteTone;
}): JSX.Element {
  const id = useId();
  return (
    <SettingRow
      title={title}
      titleId={`${id}-title`}
      description={description}
      descriptionId={`${id}-why`}
      icon={icon}
      on={checked && !disabled}
      testId={testId}
      control={
        <Switch checked={checked} disabled={disabled} onChange={onChange} labelledBy={`${id}-title`} describedBy={`${id}-why`} testId={testId ? `${testId}-switch` : undefined} />
      }
    >
      {note && <RowNote tone={noteTone}>{note}</RowNote>}
    </SettingRow>
  );
}

export type RowNoteTone = "info" | "warning";

/** A line under a row's words: what just happened (info), or something
 * to look at (warning: the app's warning icon and tone, as a notice's). */
export function RowNote({ tone = "info", children, testId }: { tone?: RowNoteTone; children: ReactNode; testId?: string }): JSX.Element {
  if (tone === "warning") {
    return (
      <span className="mt-1 flex items-start gap-1.5 text-label text-warning" role="status" data-tone="warning" data-testid={testId}>
        <IconWarn aria-hidden="true" width={14} height={14} className="mt-px shrink-0" />
        <span>{children}</span>
      </span>
    );
  }
  return (
    <span className="text-label text-ink-2" role="status" data-testid={testId}>
      {children}
    </span>
  );
}

/** The switch: a real `role="switch"` button, always labelled. */
export function Switch({
  checked,
  disabled = false,
  onChange,
  labelledBy,
  describedBy,
  label,
  testId,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  labelledBy?: string;
  describedBy?: string;
  /** When nothing on screen names it. */
  label?: string;
  testId?: string;
}): JSX.Element {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={
        // AJ-E1558: the kit's switch (ToggleRow's): the accent's fill
        // while on, the surface's third step with a quiet rim while off.
        "relative h-5 w-[34px] shrink-0 rounded-full transition-colors disabled:opacity-40 " +
        (checked ? "bg-accent-fill" : "bg-surface-3 inset-ring-1 inset-ring-hairline")
      }
      data-testid={testId}
    >
      <span
        aria-hidden="true"
        className={"absolute top-[3px] size-3.5 rounded-full transition-[left] " + (checked ? "left-[17px] bg-white" : "left-[3px] bg-faint")}
      />
    </button>
  );
}

/** A pane's 24 px tile and its icon (AJ-E1558, PAT-COLOR: the brand
 * violet means navigation): the accent's soft step on every pane, so the
 * icons are what tell them apart; the pane shown takes the strong step,
 * the accent's fill with a white glyph. */
export function PaneTile({ current = false, children }: { current?: boolean; children: ReactNode }): JSX.Element {
  return (
    <span aria-hidden="true" data-current={current || undefined} className={"flex size-6 shrink-0 items-center justify-center rounded-md transition-colors " + (current ? "bg-accent-fill text-white" : "bg-accent-soft text-accent-hi")}>
      {children}
    </span>
  );
}
