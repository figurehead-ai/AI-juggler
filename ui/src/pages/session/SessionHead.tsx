/**
 * AJ-E2108 — a session page's header parts: its app's icon with the
 * profile's badge cut into its corner (the row anatomy's mark, larger),
 * the chips (its state first, then where it runs, profile, account,
 * folder), and Go to it. One set for every kind of session.
 */
import { useEffect, useId, useRef, useState, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Button, type ChipTone } from "../../components/primitives";
import { ProfileBadge, markPlace } from "../../components/PlaceMarks";
import { AppIconTile } from "../../components/AppIcon";
import { profileColor } from "../../lib/profile-marks";
import { IconGoTo } from "../../components/icons-actions";
import { ToolMark } from "../../components/ToolMark";
import { shortPath } from "../../screens/AccountManage";
import { ago, agoWords } from "../../lib/time-words";
import { accountLine, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import { appWords, type SessionModel } from "./model";

/** The profile's name as the rail's badge reads it (its place's, else
 * its folder's): one colour and monogram everywhere. */
function profileName(model: SessionModel): string {
  const place = model.places[0];
  return place?.profile ?? place?.instance ?? model.profileLabel;
}

/** The app's icon in the header: the one place to swap in the app's
 * real icon (read from the Mac at run time) when it comes. Until then
 * the app's mark from the place registry, or, not open in any app now,
 * its tool's own mark (never "An app"). */
export function SessionAppIcon({ model, size = 48 }: { model: SessionModel; size?: number }): JSX.Element {
  if (model.places.length === 0 && model.sessionId === null) {
    return (
      <span className="inline-flex items-center justify-center rounded-[28%] border border-hairline bg-surface-3 text-ink" style={{ width: size, height: size }} data-testid="session-app-icon">
        <ToolMark tool={model.tool} size={Math.round(size / 2)} />
      </span>
    );
  }
  return (
    <span className="inline-flex" data-testid="session-app-icon">
      {(() => {
        const place = markPlace(model.places[0], model.sessionId !== null);
        return <AppIconTile appId={place.appId} appPid={place.appPid} name={place.app} size={size} />;
      })()}
    </span>
  );
}

/** The header's mark: one app icon, the profile's badge (its colour and
 * monogram, the rail's) beside it, never over it. Decorative: the chips
 * name both in words. */
export function SessionMark({ model }: { model: SessionModel }): JSX.Element {
  const place = model.places[0];
  const name = place?.profile ?? place?.instance ?? model.profileLabel;
  return (
    <span aria-hidden="true" className="inline-flex shrink-0 items-end gap-1.5" data-testid="session-mark">
      <SessionAppIcon model={model} />
      {name && (
        <span data-testid="session-mark-profile" title={name} className="-mb-0.5">
          <ProfileBadge name={name} size={18} />
        </span>
      )}
    </span>
  );
}

/** Where it runs, in words: "Running in the Claude app · viewed 2 min
 * ago", never "in front" (D-37: the app's own record, said as such). */
export function whereWords(model: SessionModel, now: number): string {
  if (!model.live) {
    return model.state.since !== null ? `Last active ${agoWords(model.state.since, now)}` : "Not open now";
  }
  const place = model.places[0];
  const app = appWords(place, model.tool);
  const where = app ? `Running in ${app}` : model.sessionId ? "Running in AI Juggler" : "Running";
  const more = model.places.length > 1 ? ` (and ${model.places.length - 1} more)` : "";
  return model.viewedMs !== null ? `${where}${more} · viewed ${agoWords(model.viewedMs, now)}` : `${where}${more}`;
}

const TONES: Record<ChipTone, string> = {
  neutral: "border-hairline bg-surface-2 text-ink",
  success: "border-success/30 bg-success-soft text-success",
  warning: "border-warning/35 bg-warning-soft text-warning",
  danger: "border-danger/30 bg-danger-soft text-danger",
  live: "border-live/30 bg-live-soft text-live",
  accent: "border-accent/30 bg-accent-soft text-accent-hi",
};

/** One header chip, 13 px (the readability scale's meta size). One line
 * always: a long one ends in an ellipsis, all of it on hover. */
function HeadChip({ tone = "neutral", strong = false, title, children, testId, className = "", wide = false }: { tone?: ChipTone; strong?: boolean; title?: string; children: ReactNode; testId: string; className?: string; /** Shown only where the header has room (60 rem). */ wide?: boolean }): JSX.Element {
  return (
    <span
      data-testid={testId}
      data-tone={tone}
      title={title}
      className={(wide ? "hidden @[60rem]:inline-flex" : "inline-flex") + " min-w-0 max-w-full items-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-0.5 text-meta leading-5 " + (strong ? "font-semibold " : "") + TONES[tone] + " " + className}
    >
      {children}
    </span>
  );
}

/** The chips a narrow header folds away (account, folder), behind "+2":
 * named on hover, listed in a small panel on a click or Enter. */
function MoreChips({ rows }: { rows: [string, string, boolean][] }): JSX.Element | null {
  const [open, setOpen] = useState(false);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  const button = useRef<HTMLButtonElement | null>(null);
  const panel = useRef<HTMLDivElement | null>(null);
  const id = `session-more-chips-${useId().replace(/:/g, "")}`;
  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!panel.current?.contains(target) && !button.current?.contains(target)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setOpen(false);
      button.current?.focus();
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);
  if (rows.length === 0) return null;
  const words = rows.map(([label, value]) => `${label}: ${value}`).join("\n");
  return (
    <>
      <button
        ref={button}
        type="button"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        aria-label={`+${rows.length} more: its ${rows.map(([label]) => label.toLowerCase()).join(" and ")}`}
        title={words}
        onClick={() => {
          const rect = button.current?.getBoundingClientRect();
          if (rect) setPlace({ left: Math.max(8, Math.min(rect.left, window.innerWidth - 328)), top: rect.bottom + 6 });
          setOpen((now) => !now);
        }}
        data-testid="session-more-chips"
        className="inline-flex h-6 shrink-0 items-center rounded-full border border-hairline bg-surface-2 px-2 text-meta leading-5 text-ink outline-none transition-colors hover:bg-surface-3 focus-visible:ring-1 focus-visible:ring-accent/60 @[60rem]:hidden"
      >
        {/* "+2": how many more, unlike the page's own ⋯ menu beside it; its
            name starts with the same words (WCAG 2.5.3). */}
        <span className="tabular-nums">+{rows.length}</span>
      </button>
      {open &&
        place &&
        createPortal(
          <div
            ref={panel}
            id={id}
            role="group"
            aria-label="Where it runs"
            style={{ left: place.left, top: place.top }}
            className="fixed z-50 flex w-80 flex-col gap-2 rounded-xl border border-hairline bg-surface p-3 shadow-card"
            data-testid="session-more-chips-panel"
          >
            {rows.map(([label, value, mono]) => (
              <div key={label} className="flex min-w-0 flex-col gap-0.5">
                <span className="text-label text-muted">{label}</span>
                <span className={"break-words text-meta text-ink " + (mono ? "font-mono" : "")}>{value}</span>
              </div>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
}

/** The header's chips, on one line at any width: its state, where it
 * runs and its profile always; its account and folder too where the
 * header has room (60 rem), else behind "+2" (and in Where it runs). */
export function SessionChips({ model, account, now }: { model: SessionModel; account: SubscriptionInfo | undefined; now: number }): JSX.Element {
  const since = model.live && model.state.since !== null ? ` · ${ago(model.state.since, now)}` : "";
  const line = account ? accountLine(account) : null;
  const where = whereWords(model, now);
  const accountWords = account && account.state === "signed_in" ? `${accountName(account)}${line ? ` · ${line}` : ""}` : null;
  const folded: [string, string, boolean][] = [
    ...(accountWords ? ([["Account", accountWords, false]] as [string, string, boolean][]) : []),
    ...(model.cwd ? ([["Folder", shortPath(model.cwd), true]] as [string, string, boolean][]) : []),
  ];
  return (
    <>
      <HeadChip testId="session-state-chip" tone={model.state.tone} strong className="shrink-0">
        {model.live && <span aria-hidden="true" className={"size-2 rounded-full bg-current " + (model.state.working ? "motion-safe:animate-pulse" : "")} />}
        {model.state.words}
        {model.state.asks || model.state.tone === "accent" ? since : ""}
      </HeadChip>
      <HeadChip testId="session-where-chip" title={where} className="shrink">
        <span className="truncate">{where}</span>
      </HeadChip>
      <span
        data-testid="session-profile-chip"
        title={`Profile: ${model.profileLabel}`}
        className="inline-flex min-w-0 max-w-full shrink items-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-0.5 text-meta font-medium leading-5 text-ink"
        style={{ borderColor: profileColor(profileName(model)) }}
      >
        <ProfileBadge name={profileName(model)} size={16} />
        <span className="truncate">Profile: {model.profileLabel}</span>
      </span>
      {accountWords && (
        <HeadChip testId="session-account-chip" title={accountWords} wide className="shrink-[2]">
          <span className="truncate">Account: {accountWords}</span>
        </HeadChip>
      )}
      {model.cwd && (
        <HeadChip testId="session-folder-chip" title={model.cwd} wide className="shrink-[3]">
          <span className="truncate font-mono text-meta">{shortPath(model.cwd)}</span>
        </HeadChip>
      )}
      <MoreChips rows={folded} />
    </>
  );
}

/** "Go to it ↵": the page's primary while it runs somewhere. Enter on the
 * page (not in a field or a terminal) does the same. */
export function GoToButton({ onGo, why }: { onGo: () => void; why: string | null }): JSX.Element {
  useEffect(() => {
    if (why !== null) return;
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const here = target === document.body || target?.closest?.("[data-testid='page-header']") !== null;
      if (event.key !== "Enter" || event.metaKey || event.altKey || event.ctrlKey || event.shiftKey || !here) return;
      if (target?.closest?.("button, a, input, textarea, select, [contenteditable='true']")) return;
      event.preventDefault();
      onGo();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onGo, why]);
  return (
    <Button size="md" variant="primary" disabled={why !== null} title={why ?? "Go to it (↵)"} onClick={onGo} data-testid="session-go-to">
      Go to it
      <IconGoTo width={14} height={14} aria-hidden="true" />
      <kbd aria-hidden="true" className="font-sans text-label opacity-80">
        ↵
      </kbd>
    </Button>
  );
}
