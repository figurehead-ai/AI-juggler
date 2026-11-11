/**
 * AJ-E1504 — `?` (outside a text field) opens every shortcut, by area.
 * It is rendered from the registry in `lib/shortcuts.ts`, the same list
 * the keys are matched against, so it can't drift from what they do.
 * Fleet's keys show once Fleet does (the 0.2 walk, F18).
 */
import type { JSX } from "react";
import { Modal } from "./form";
import { MoreBelow, useMoreBelow } from "./MoreBelow";
import { changedKey, keysOf, SHORTCUT_AREAS, SHORTCUTS, type Shortcut, type ShortcutId } from "../lib/shortcuts";

/** One key, as a key cap (the tier tag's quiet border). */
export function Keycap({ children }: { children: string }): JSX.Element {
  return (
    <kbd className="shrink-0 whitespace-nowrap rounded border border-hairline bg-surface-2 px-1.5 py-0.5 font-mono text-label text-ink">{children}</kbd>
  );
}

/** The sheet's shortcuts: Fleet's only while Fleet shows. */
export function shownShortcuts(fleet: boolean): readonly Shortcut[] {
  return (SHORTCUTS as readonly Shortcut[]).filter((entry) => fleet || !entry.fleet);
}

export function ShortcutSheet({ onClose, fleet = false }: { onClose: () => void; fleet?: boolean }): JSX.Element {
  const shown = shownShortcuts(fleet);
  // 0.3.2: the list runs past a short window; say so under it.
  const [box, moreBelow, onScroll] = useMoreBelow();
  return (
    <Modal
      title="Keyboard shortcuts"
      subtitle="In a text field or the terminal, only the ⌘ ones work."
      onClose={onClose}
      broad
      testId="shortcut-sheet"
    >
      {/* 0.4.1: columns only where they fit (one when narrow), a stable
          gutter so a scrollbar never covers the keys, labels that wrap
          and key chips that never shrink: nothing is clipped. */}
      <div
        ref={box}
        onScroll={onScroll}
        className="grid max-h-[60vh] grid-cols-[repeat(auto-fit,minmax(min(100%,19rem),1fr))] gap-x-8 gap-y-5 overflow-y-auto overflow-x-hidden pr-3 [scrollbar-gutter:stable]"
        tabIndex={0}
        aria-label="Shortcuts by area"
      >
        {SHORTCUT_AREAS.map((area) => (
          <section key={area} aria-labelledby={`shortcuts-${area}`} className="flex min-w-0 flex-col gap-1.5">
            <h3 id={`shortcuts-${area}`} className="text-label font-semibold text-muted">
              {area}
            </h3>
            <ul className="flex flex-col">
              {shown.filter((entry) => entry.area === area).map((entry) => (
                <li
                  key={entry.id}
                  data-shortcut={entry.id}
                  className="flex items-center justify-between gap-3 border-b border-hairline/60 py-1.5 text-xs last:border-b-0"
                >
                  <span className="min-w-0 flex-1 text-ink [overflow-wrap:anywhere]">{entry.label}</span>
                  <span className="flex shrink-0 flex-wrap items-center justify-end gap-1">
                    {/* AJ-E2905: a key you changed says so. */}
                    {changedKey(entry.id as ShortcutId) && <span className="text-label text-muted" data-testid="shortcut-changed">changed</span>}
                    {keysOf(entry.id as ShortcutId).map((keys, index) => (
                      <span key={keys} className="flex items-center gap-1">
                        {index > 0 && <span className="text-label text-muted">or</span>}
                        <Keycap>{keys}</Keycap>
                      </span>
                    ))}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
      {moreBelow && <MoreBelow box={box} className="mx-auto mt-2 flex" testId="shortcut-sheet-more" />}
    </Modal>
  );
}
