/**
 * AJ-E1504 — `?` (outside a text field) opens every shortcut, by area.
 * It is rendered from the registry in `lib/shortcuts.ts`, the same list
 * the keys are matched against, so it can't drift from what they do.
 * Fleet's keys show once Fleet does (the 0.2 walk, F18).
 */
import type { JSX } from "react";
import { Modal } from "./form";
import { SHORTCUT_AREAS, SHORTCUTS, type Shortcut } from "../lib/shortcuts";

/** One key, as a key cap (the tier tag's quiet border). */
export function Keycap({ children }: { children: string }): JSX.Element {
  return (
    <kbd className="rounded border border-hairline bg-surface-2 px-1.5 py-0.5 font-mono text-label text-ink">{children}</kbd>
  );
}

/** The sheet's shortcuts: Fleet's only while Fleet shows. */
export function shownShortcuts(fleet: boolean): readonly Shortcut[] {
  return (SHORTCUTS as readonly Shortcut[]).filter((entry) => fleet || !entry.fleet);
}

export function ShortcutSheet({ onClose, fleet = false }: { onClose: () => void; fleet?: boolean }): JSX.Element {
  const shown = shownShortcuts(fleet);
  return (
    <Modal
      title="Keyboard shortcuts"
      subtitle="In a text field or the terminal, only the ⌘ ones work."
      onClose={onClose}
      wide
      testId="shortcut-sheet"
    >
      <div className="grid max-h-[60vh] gap-x-8 gap-y-5 overflow-y-auto pr-1 sm:grid-cols-2" tabIndex={0} aria-label="Shortcuts by area">
        {SHORTCUT_AREAS.map((area) => (
          <section key={area} aria-labelledby={`shortcuts-${area}`} className="flex flex-col gap-1.5">
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
                  <span className="text-ink">{entry.label}</span>
                  <span className="flex shrink-0 items-center gap-1">
                    {entry.keys.map((keys, index) => (
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
    </Modal>
  );
}
