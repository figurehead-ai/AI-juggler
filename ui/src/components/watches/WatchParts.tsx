/**
 * AJ-E3816 — the Watches view's small parts: a watch's target mark (its
 * App's own icon, read at run time, or a kind's icon; two targets as two
 * overlapping marks), the echo with its names in bold, a chip, the state
 * column and the section fold.
 */
import type { JSX, ReactNode } from "react";
import { LogoTile, ToolLogo } from "../ToolLogo";
import type { EchoSpan } from "../../types/generated/EchoSpan";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import type { StateTone, TargetOption } from "./watches-view";
import { IconChevronD, IconChevronR, IconFolderShut, IconPause, IconPersonOutline, IconPoolRing, IconTiles, type WatchIcon } from "./watch-icons";

const KIND_ICONS: Record<WatchTarget["kind"], WatchIcon> = {
  session: IconTiles,
  app: IconTiles,
  project: IconFolderShut,
  pool: IconPoolRing,
  account: IconPersonOutline,
  any: IconTiles,
};

/** One target's mark: its tool's icon, else its kind's. */
export function TargetMark({ option, size, ghost = false }: { option: TargetOption; size: 24 | 32 | 40; ghost?: boolean }): JSX.Element {
  if (ghost) {
    const Icon = KIND_ICONS[option.kind];
    const glyph = Math.round(size * 0.46);
    return (
      <span aria-hidden="true" className="flex shrink-0 items-center justify-center rounded-[10px] text-faint inset-ring-[1.5px] inset-ring-hairline" style={{ width: size, height: size }} data-testid="watch-mark-ghost">
        <Icon width={glyph} height={glyph} />
      </span>
    );
  }
  if (option.tool) return <ToolLogo tool={option.tool} size={size} name={option.name} />;
  return <LogoTile icon={KIND_ICONS[option.kind]} size={size} />;
}

/** A watch's mark: one target's, or the first two overlapping. */
export function WatchMarks({ options, compact = false, ghost = false, dim = false }: { options: TargetOption[]; compact?: boolean; ghost?: boolean; dim?: boolean }): JSX.Element {
  const [first, second] = options;
  const box = compact ? "w-8 h-7" : "w-11 h-9";
  if (!first) return <span className={box} />;
  return (
    <span aria-hidden="true" className={"relative flex shrink-0 items-center " + box + (dim ? " opacity-60" : "")} data-testid="watch-marks">
      {second ? (
        <>
          <span className="absolute left-0 top-0">
            <TargetMark option={first} size={24} ghost={ghost} />
          </span>
          <span className={"absolute rounded-[8px] shadow-[0_0_0_2px_var(--color-surface)] " + (compact ? "left-3 top-2" : "left-[18px] top-3")}>
            <TargetMark option={second} size={24} ghost={ghost} />
          </span>
        </>
      ) : (
        <TargetMark option={first} size={compact ? 24 : 32} ghost={ghost} />
      )}
    </span>
  );
}

/** The echo's first sentence, its names in bold (the code's own words). */
export function EchoHead({ spans, tone = "live" }: { spans: readonly EchoSpan[]; tone?: "live" | "quiet" }): JSX.Element {
  return (
    <>
      {spans.map((span, i) =>
        span.name ? (
          <b key={i} className={tone === "live" ? "font-semibold text-ink" : "font-medium text-ink-2"}>
            {span.text}
          </b>
        ) : (
          <span key={i}>{span.text}</span>
        ),
      )}
    </>
  );
}

/** A small chip: a lifetime, or "Even during quiet hours" (accent). */
export function WatchChip({ icon: Icon, accent = false, children, testId }: { icon?: WatchIcon; accent?: boolean; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <span className={"inline-flex h-[22px] items-center gap-1.5 whitespace-nowrap rounded-full px-2 text-label font-medium " + (accent ? "bg-accent-soft text-accent-hi" : "bg-surface-3 text-ink-2")} data-testid={testId}>
      {Icon && <Icon width={13} height={13} aria-hidden="true" className={accent ? "text-accent-hi" : "text-faint"} />}
      {children}
    </span>
  );
}

/** The state column: a dot (green once it has told you, grey while it
 * waits; never amber or red: a watch is calm) and two lines. */
export function StateCell({ tone, top, sub }: { tone: StateTone; top: string; sub: string }): JSX.Element {
  return (
    <span className="flex min-w-0 flex-col gap-px text-meta" data-testid="watch-state" data-tone={tone}>
      <span className="flex items-center gap-[7px] font-medium text-ink">
        {tone === "paused" ? <IconPause width={12} height={12} aria-hidden="true" className="text-faint" /> : <span aria-hidden="true" className={"size-[7px] shrink-0 rounded-full " + (tone === "told" ? "bg-success" : "bg-faint")} />}
        {top}
      </span>
      <span className="truncate text-muted" title={sub}>{sub}</span>
    </span>
  );
}

/** A section's fold: a chevron, its title and a quiet count. */
export function FoldHead({ open, onToggle, title, line, controls, testId }: { open: boolean; onToggle: () => void; title: string; line: string; controls: string; testId?: string }): JSX.Element {
  const Chevron = open ? IconChevronD : IconChevronR;
  return (
    <h3 className="m-0">
      <button type="button" aria-expanded={open} aria-controls={controls} onClick={onToggle} className="flex items-center gap-2.5 rounded-lg px-1 py-1 outline-none focus-visible:ring-2 focus-visible:ring-accent/60" data-testid={testId}>
        <Chevron width={16} height={16} aria-hidden="true" className="text-faint" />
        <span className="text-section font-semibold text-ink">{title}</span>
        <span className="text-meta text-muted">{line}</span>
      </button>
    </h3>
  );
}
