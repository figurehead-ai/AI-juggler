/**
 * AJ-E1547 (PAT-OSP-2) — a picture of the system's dialog, drawn from its
 * own words (never a screenshot of the real one), so the user knows it
 * when it appears and which button to click. Theme-aware: the app's
 * tokens draw it in both themes. An alert is a centred card with the
 * app's icon, its title, its text and its buttons; a banner is the
 * notice in the corner of the screen. The button to click is marked.
 *
 * One picture, one name: screen readers hear "The system's dialog: …,
 * click …" and nothing inside it.
 */
import type { JSX } from "react";
import { IconJuggler } from "./icons";
import type { OsPromptDialogInfo } from "../lib/ipc/os-prompts";

/** The app's mark as the system's dialog shows it: its icon, small. */
function AppIcon({ size }: { size: number }): JSX.Element {
  return (
    <span className="inline-flex shrink-0 items-center justify-center rounded-[22%] border border-hairline bg-surface-2 text-accent" style={{ width: size, height: size }}>
      <IconJuggler aria-hidden="true" width={size * 0.72} height={size * 0.72} />
    </span>
  );
}

function DialogButton({ label, marked }: { label: string; marked: boolean }): JSX.Element {
  return (
    <span
      className={
        "relative inline-flex min-w-[72px] items-center justify-center rounded-md px-3 py-1 text-xs font-medium " +
        (marked ? "border border-accent-hi/40 bg-accent-fill text-white ring-2 ring-accent ring-offset-2 ring-offset-surface" : "border border-hairline bg-surface-2 text-ink-2")
      }
      data-marked={marked ? "1" : undefined}
    >
      {label}
    </span>
  );
}

export function OsPromptPicture({ dialog, className = "" }: { dialog: OsPromptDialogInfo; className?: string }): JSX.Element {
  const name = `The system's dialog: ${dialog.title}${dialog.click ? ` Click ${dialog.click}.` : ""}`;
  const marked = (label: string) => label === dialog.click;
  if (dialog.style === "banner") {
    return (
      // eslint-disable-next-line jsx-a11y/prefer-tag-over-role -- a picture drawn from HTML parts gets one name for the whole (WAI-ARIA img); <img> can't hold children
      <div role="img" aria-label={name} className={`flex justify-end rounded-card border border-hairline bg-canvas p-3 ${className}`} data-testid="os-prompt-picture" data-style="banner">
        <div aria-hidden="true" className="flex w-full max-w-[340px] items-start gap-2.5 rounded-xl border border-hairline bg-surface p-3 shadow-card">
          <AppIcon size={28} />
          <div className="min-w-0 flex-1">
            <p className="text-xs font-semibold text-ink">{dialog.title}</p>
            <p className="mt-0.5 text-label leading-snug text-ink-2">{dialog.body}</p>
            {dialog.buttons.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-1.5">
                {dialog.buttons.map((label) => (
                  <DialogButton key={label} label={label} marked={marked(label)} />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }
  return (
    // eslint-disable-next-line jsx-a11y/prefer-tag-over-role -- as above: one name for the drawn picture
    <div role="img" aria-label={name} className={`flex justify-center rounded-card border border-hairline bg-canvas px-3 py-4 ${className}`} data-testid="os-prompt-picture" data-style="alert">
      <div aria-hidden="true" className="flex w-full max-w-[280px] flex-col items-center gap-2 rounded-xl border border-hairline bg-surface p-4 text-center shadow-card">
        <AppIcon size={40} />
        <p className="text-xs font-semibold leading-snug text-ink">{dialog.title}</p>
        {dialog.body && <p className="text-label leading-snug text-ink-2">{dialog.body}</p>}
        <div className="mt-1 flex w-full flex-wrap justify-center gap-1.5">
          {dialog.buttons.map((label) => (
            <DialogButton key={label} label={label} marked={marked(label)} />
          ))}
        </div>
      </div>
    </div>
  );
}
