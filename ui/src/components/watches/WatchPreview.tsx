/**
 * AJ-E3816 — "Looks like this", beside the form: the echo, large, word
 * for word what AI Juggler watches (when editing, the words it had struck
 * through above, so the change is seen before saving: ADR-0036 §2.4 step
 * 5); the notification as the Mac will show it; what happens (where it
 * shows, quiet hours, how it ends, the last marked as what changes most);
 * and that a watch only tells you. Under 960 px it folds into a strip.
 */
import type { JSX, ReactNode } from "react";
import type { EchoParts } from "../../types/generated/EchoParts";
import { EchoHead } from "./WatchParts";
import { joinedWords } from "./watch-form-state";
import { PREVIEW } from "./watches-words";
import { IconJuggler } from "../icons";
import { IconCheckCircle, IconHourglass, IconInfinity, IconInfoCircle, IconLines, IconMessage, IconMoon, IconShieldLine, IconSun, type WatchIcon } from "./watch-icons";

export interface PreviewFacts {
  echo: EchoParts | null;
  /** The words it had, when editing and they changed. */
  was: EchoParts | null;
  /** The notification's title ("“api” finished · your watch"). */
  title: string | null;
  /** Its line ("Agent CLI · shop · open it"). */
  line: string | null;
  /** Where it shows. */
  shows: string[];
  /** Quiet hours, in words; null when it doesn't apply. */
  quiet: string | null;
  quietWakes: boolean;
  /** How it ends. */
  ends: string;
  endsKind: "once" | "until_ends" | "today" | "until" | "always";
}

const END_ICONS: Record<PreviewFacts["endsKind"], WatchIcon> = { once: IconInfoCircle, until_ends: IconHourglass, today: IconSun, until: IconSun, always: IconInfinity };

function Row({ icon: Icon, marked = false, children, testId }: { icon: WatchIcon; marked?: boolean; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <li className={"flex items-start gap-2.5 rounded-lg px-2.5 py-1.5 text-meta text-ink-2 " + (marked ? "bg-accent-soft shadow-[inset_3px_0_0_var(--color-accent)]" : "")} data-testid={testId}>
      <Icon width={15} height={15} aria-hidden="true" className={"mt-px shrink-0 " + (marked ? "text-accent-hi" : "text-faint")} />
      <span>{children}</span>
    </li>
  );
}

export function WatchPreview({ facts }: { facts: PreviewFacts }): JSX.Element {
  return (
    <aside aria-label={PREVIEW.title} className="overflow-hidden rounded-card bg-surface shadow-card inset-ring-1 inset-ring-hairline/75" data-testid="watch-preview">
      <div className="flex items-center gap-2 px-4 pb-2.5 pt-3.5">
        <span className="flex-1 text-label font-medium uppercase tracking-[0.07em] text-section-label">{PREVIEW.title}</span>
        <span className="text-label text-muted">{PREVIEW.updates}</span>
      </div>
      <div className="mx-3 rounded-xl bg-[color-mix(in_srgb,var(--color-accent)_6%,var(--color-surface))] px-3.5 pb-3 pt-3.5 inset-ring-1 inset-ring-accent/35 [[data-theme=dark]_&]:bg-[color-mix(in_srgb,var(--color-accent)_9%,var(--color-surface))]">
        {facts.was && (
          <div className="mb-2.5 border-b border-[color-mix(in_srgb,var(--color-accent)_22%,var(--color-divider))] pb-2.5 text-meta text-muted" data-testid="watch-preview-was">
            <span className="mb-1 block text-label font-medium uppercase tracking-[0.07em] text-section-label">{PREVIEW.was}</span>
            <s className="decoration-faint">{joinedWords(facts.was)}</s>
          </div>
        )}
        <p className="text-[15px] leading-normal text-ink-2" aria-live="polite" data-testid="watch-preview-echo">
          {facts.echo ? (
            <>
              <EchoHead spans={facts.echo.head} /> {facts.echo.lifetime}
              {facts.echo.quiet && ` ${facts.echo.quiet}`}
            </>
          ) : (
            <span className="text-muted">…</span>
          )}
        </p>
        <p className="mt-2 flex items-center gap-1.5 text-label text-muted">
          <IconCheckCircle width={13} height={13} aria-hidden="true" />
          {PREVIEW.exact}
        </p>
      </div>
      {facts.title && (
        <div className="mx-3 mt-3 rounded-xl bg-canvas-hi bg-[radial-gradient(color-mix(in_srgb,var(--color-ink)_8%,transparent)_1px,transparent_1.2px)] bg-[length:14px_14px] px-3.5 py-4" aria-hidden="true" data-testid="watch-preview-banner">
          <div className="flex gap-2.5 rounded-[14px] bg-surface px-3 py-[11px] shadow-[0_8px_24px_-12px_rgba(14,18,25,.3)] inset-ring-1 inset-ring-hairline [[data-theme=dark]_&]:bg-surface-2">
            <span className="flex size-[30px] shrink-0 items-center justify-center rounded-lg bg-accent-fill text-white">
              <IconJuggler width={18} height={18} />
            </span>
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="flex text-label text-muted">
                <span className="flex-1">{PREVIEW.app}</span>
                {PREVIEW.now}
              </span>
              <span className="truncate text-meta font-semibold text-ink">{facts.title}</span>
              {facts.line && <span className="truncate text-label text-ink-2">{facts.line}</span>}
            </span>
          </div>
        </div>
      )}
      <ul className="flex flex-col gap-0.5 px-2 pb-1.5 pt-2.5" data-testid="watch-preview-rows">
        {facts.shows.map((words, i) => (
          <Row key={i} icon={i === 0 ? IconLines : IconMessage}>
            {words}
          </Row>
        ))}
        {facts.quiet && <Row icon={IconMoon}>{facts.quiet}</Row>}
        <Row icon={END_ICONS[facts.endsKind]} marked testId="watch-preview-ends">
          {facts.ends}
        </Row>
      </ul>
      <p className="flex items-center gap-2 border-t border-divider px-[18px] pb-3.5 pt-2.5 text-label text-muted" data-testid="watch-preview-footer">
        <IconShieldLine width={13} height={13} aria-hidden="true" className="shrink-0" />
        {PREVIEW.footer}
      </p>
    </aside>
  );
}

/** The preview folded (under 960 px): the echo in one strip. */
export function WatchPreviewStrip({ echo }: { echo: EchoParts | null }): JSX.Element {
  return (
    <p className="rounded-xl bg-accent-soft px-3.5 py-2.5 text-meta text-ink-2 inset-ring-1 inset-ring-accent/35" data-testid="watch-preview-strip">
      <span className="mr-1.5 font-semibold text-ink">{PREVIEW.summary}</span>
      {echo ? joinedWords(echo) : "…"}
    </p>
  );
}
