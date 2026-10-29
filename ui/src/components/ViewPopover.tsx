/**
 * AJ-E2104 — the rail's View (⇧V, or its button; a dot on it when the
 * view isn't the default). 0.3, the redesign's "a real menu": the row
 * menu's pattern on one grid — a "View" header with Reset (only when
 * something changed), then Show (lines that turn on and off: icon ·
 * label · what it is · ✓) and Only (a line with its current value that
 * opens a submenu of values with ✓, never a system select). Group by
 * left for the rail's own switch (RailToolbar).
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { IconView, IconWindow } from "./icons-places";
import { IconArchive, IconComputer, IconKey, IconReplay } from "./icons-actions";
import { IconProfiles, IconProjects, IconSubtasks } from "./icons";
import { IconBackground, IconChevronRight, IconEnded, IconInactive, IconQuiet } from "./icons-controls";
import { MenuItem, MenuSeparator, MenuSubmenu, RowMenu, type MenuAt } from "./RowMenu";
import { ChoiceLine, type ControlIcon } from "./controls";
import { ONLY_KEYS, resetViewParts, setRailView, SHOW_KEYS, SHOW_LABELS, SHOW_NOTES, useRailView, viewChanged, type OnlyKey, type ShowKey } from "../lib/rail-view";

const SHOW_ICONS: Record<ShowKey, ControlIcon> = {
  inactive: IconInactive,
  quiet: IconQuiet,
  ended: IconEnded,
  background: IconBackground,
  subagents: IconSubtasks,
  otherMacs: IconComputer,
  archived: IconArchive,
};

const ONLY_LABELS: Record<OnlyKey, string> = { app: "App", profile: "Profile", account: "Account", project: "Project", mac: "Mac" };
const ONLY_ICONS: Record<OnlyKey, ControlIcon> = { app: IconWindow, profile: IconProfiles, account: IconKey, project: IconProjects, mac: IconComputer };

/** A section's name in the menu: small, muted, never a line you pick. */
function Section({ children }: { children: string }): JSX.Element {
  return <span className="px-2 pb-0.5 pt-1 text-label font-semibold text-muted">{children}</span>;
}

export function ViewPopover({ options }: { options: Record<OnlyKey, { value: string; label: string }[]> }): JSX.Element {
  const view = useRailView();
  const [at, setAt] = useState<MenuAt | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const changed = viewChanged(view);
  const open = at !== null;
  const show = () => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (!rect) return;
    buttonRef.current?.focus();
    setAt({ x: rect.left, y: rect.bottom + 6, above: rect.top - 6, right: rect.right });
  };
  // ⇧V (App's key) opens it from anywhere, or closes it.
  const toggleRef = useRef(() => {});
  toggleRef.current = () => (open ? setAt(null) : show());
  useEffect(() => {
    const toggle = () => toggleRef.current();
    window.addEventListener("aij:rail-view", toggle);
    return () => window.removeEventListener("aij:rail-view", toggle);
  }, []);
  // A press on the button while it's open closes it (the menu's own
  // outside press already did); the click that follows doesn't reopen.
  const closing = useRef(false);
  const onlyKeys = ONLY_KEYS.filter((key) => options[key].length > 1 || view.only[key] !== undefined);
  const setOnly = (key: OnlyKey, value: string | null) =>
    setRailView((current) => {
      const only = { ...current.only };
      if (value) only[key] = value;
      else delete only[key];
      return { ...current, only };
    });
  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-keyshortcuts="Shift+V"
        aria-label={changed ? "View, changed from the defaults" : "View"}
        title="View (⇧V)"
        data-testid="rail-view-button"
        onMouseDown={() => {
          closing.current = open;
        }}
        onClick={() => {
          if (closing.current) closing.current = false;
          else if (open) setAt(null);
          else show();
        }}
        className={
          "relative flex size-[30px] shrink-0 items-center justify-center rounded-lg border outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (open || changed ? "border-accent/40 bg-accent-soft text-accent-hi" : "border-hairline text-muted hover:bg-surface hover:text-ink")
        }
      >
        <IconView width={14} height={14} />
        {changed && <span aria-hidden="true" data-testid="rail-view-dot" className="absolute right-1 top-1 size-1.5 rounded-full bg-accent" />}
      </button>
      {at && (
        <RowMenu heading="" ariaLabel="View" at={at} minWidth={300} focusFirst='[role="menuitemcheckbox"]' testId="rail-view" onClose={() => setAt(null)}>
          <div className="flex min-h-7 items-center justify-between gap-2 px-2 pb-1">
            <span className="text-meta font-semibold text-ink">View</span>
            {changed && (
              <button
                type="button"
                role="menuitem"
                data-testid="rail-view-reset"
                onClick={resetViewParts}
                className="-mr-1 flex items-center gap-1 rounded-md px-1.5 py-0.5 text-label font-medium text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
              >
                <IconReplay width={12} height={12} aria-hidden="true" />
                Reset
              </button>
            )}
          </div>
          <Section>Show</Section>
          {SHOW_KEYS.map((key) => (
            <ShowLine key={key} which={key} on={view.show[key]} />
          ))}
          {onlyKeys.length > 0 && (
            <>
              <MenuSeparator />
              <Section>Only</Section>
              {onlyKeys.map((key) => {
                const picked = view.only[key];
                const value = picked === undefined ? "All" : (options[key].find((option) => option.value === picked)?.label ?? picked);
                return (
                  <MenuSubmenu
                    key={key}
                    label={`Only this ${ONLY_LABELS[key].toLowerCase()}`}
                    testId={`rail-view-only-${key}`}
                    line={
                      <ChoiceLine
                        icon={ONLY_ICONS[key]}
                        label={ONLY_LABELS[key]}
                        end={
                          <span className="flex min-w-0 max-w-36 items-center gap-0.5 text-label text-muted">
                            <span className="truncate">{value}</span>
                            <IconChevronRight width={12} height={12} strokeWidth={2} aria-hidden="true" className="shrink-0" />
                          </span>
                        }
                      />
                    }
                  >
                    <MenuItem checked={picked === undefined} testId={`rail-view-only-${key}-all`} onClick={() => setOnly(key, null)}>
                      <ChoiceLine label="All" checked={picked === undefined} strong={picked === undefined} />
                    </MenuItem>
                    {options[key].map((option) => (
                      <MenuItem key={option.value} checked={picked === option.value} testId={`rail-view-only-${key}-${option.value}`} onClick={() => setOnly(key, option.value)}>
                        <ChoiceLine label={option.label} checked={picked === option.value} strong={picked === option.value} />
                      </MenuItem>
                    ))}
                  </MenuSubmenu>
                );
              })}
            </>
          )}
        </RowMenu>
      )}
    </>
  );
}

/** One Show line: it turns on and off, the menu stays open. Named by
 * its label; what it is, under it, is its description. */
function ShowLine({ which, on }: { which: ShowKey; on: boolean }): JSX.Element {
  const note = SHOW_NOTES[which];
  const noteId = `rail-view-note-${which}`;
  return (
    <MenuItem
      checked={on}
      multi
      testId={`rail-view-show-${which}`}
      describedBy={note ? noteId : undefined}
      onClick={() => setRailView((current) => ({ ...current, show: { ...current.show, [which]: !current.show[which] } }))}
    >
      <ChoiceLine
        icon={SHOW_ICONS[which]}
        label={SHOW_LABELS[which]}
        hint={
          note && (
            <span id={noteId} aria-hidden="true">
              {note}
            </span>
          )
        }
        checked={on}
      />
    </MenuItem>
  );
}
