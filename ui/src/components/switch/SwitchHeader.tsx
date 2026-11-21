/**
 * 0.5.0 (D-142, DESIGN-SYSTEM §8): every switch dialog's fixed header, on
 * every step and phase: an icon tile, the title, the conversation and
 * where it is now, a from → to strip with each side's logo (the one
 * picture of what happens), and a wizard's step marks under it.
 */
import { useState, type JSX, type ReactNode, type SVGProps } from "react";
import type { PreflightInfoData } from "../../lib/ipc/switch-preflight";
import { cliAppName } from "../../lib/app-name";
import { IconProfiles, IconStateHandover } from "../icons";
import { LogoTile, ToolLogo } from "../ToolLogo";
import { destinationsOf, type Destination } from "./WhereChoice";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

/** One side of the strip: its logo and its name. */
export interface Side {
  name: string;
  logo: ReactNode;
}

export function SwitchHeader({
  icon,
  title,
  sub,
  from,
  to,
  marks,
  testId,
  titleTestId,
  stripTestId,
}: {
  icon: Icon;
  title: string;
  /** The conversation and where it is now. */
  sub: string;
  from: Side;
  to: Side;
  /** A wizard's step marks. */
  marks?: ReactNode;
  testId: string;
  titleTestId: string;
  stripTestId: string;
}): JSX.Element {
  return (
    <div className="flex flex-col gap-3" data-testid={testId}>
      <div className="flex items-start gap-3">
        <LogoTile icon={icon} size={40} on />
        <div className="flex min-w-0 flex-col gap-0.5">
          <h2 className="type-section" data-testid={titleTestId}>
            {title}
          </h2>
          <p className="type-help truncate" title={sub}>
            {sub}
          </p>
        </div>
      </div>
      <div className="flex min-w-0 flex-wrap items-center gap-2 text-meta font-medium text-ink" data-testid={stripTestId}>
        <span className="inline-flex min-w-0 items-center gap-2 rounded-full bg-surface-2 py-1 pr-3 pl-1">
          {from.logo}
          <span className="truncate">{from.name}</span>
        </span>
        <svg aria-hidden="true" width="40" height="12" viewBox="0 0 40 12" className="shrink-0 text-faint">
          <path d="M1 6h36M32 1.5 37 6l-5 4.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        <span className="sr-only">to</span>
        <span className="inline-flex min-w-0 items-center gap-2 rounded-full bg-accent-soft py-1 pr-3 pl-1">
          {to.logo}
          <span className="truncate">{to.name}</span>
        </span>
      </div>
      {marks}
    </div>
  );
}

/** A copy switch's header (the switch's own, where Move doesn't apply):
 * "Switch to Personal", the conversation and the account it is on now,
 * and from → to, the App it continues in following Desktop or CLI as
 * chosen. Before the check answers, the target alone. */
export function CopyHeader({ preflight, target, destination, marks }: { preflight: PreflightInfoData | null; target: string; destination: Destination; marks?: ReactNode }): JSX.Element {
  const desktop = preflight && destination === "desktop" ? destinationsOf(preflight).desktop : null;
  const tool = preflight?.target_tool ?? null;
  const to = desktop?.name ?? (tool ? cliAppName(tool, target) : target);
  const source = preflight?.source_account ?? "this account";
  return (
    <SwitchHeader
      icon={IconStateHandover}
      title={`Switch to ${target}`}
      sub={`${preflight?.source_title ?? "This conversation"} · now on ${source}`}
      from={{ name: source, logo: <LogoTile icon={IconProfiles} size={24} /> }}
      to={{ name: to, logo: tool ? <ToolLogo tool={tool} kind={destination === "desktop" ? "desktop" : "cli"} appId={desktop?.app_id} size={24} /> : <LogoTile icon={IconProfiles} size={24} on /> }}
      marks={marks}
      testId="switch-copy-header"
      titleTestId="switch-title"
      stripTestId="switch-strip"
    />
  );
}

/** The last check's answer: kept while a phase that holds none (the
 * switch running) shows. */
export function useLastCheck(checked: PreflightInfoData | null): PreflightInfoData | null {
  const [seen, setSeen] = useState<PreflightInfoData | null>(checked);
  // Kept as it changes (React's "adjusting state on a prop change").
  if (checked && checked !== seen) setSeen(checked);
  return checked ?? seen;
}

/** The switch dialog's header: Move's "Continue on <App>" where moving
 * applies, else the copy's. */
export function FlowHeader({ move, marks, target, preflight, destination }: { move: { offer: unknown; header: (marks: ReactNode, target: string, preflight: PreflightInfoData) => ReactNode }; marks: ReactNode; target: string; preflight: PreflightInfoData | null; destination: Destination }): JSX.Element {
  if (move.offer && preflight) return <>{move.header(marks, target, preflight)}</>;
  return <CopyHeader preflight={preflight} target={target} destination={destination} marks={marks} />;
}
