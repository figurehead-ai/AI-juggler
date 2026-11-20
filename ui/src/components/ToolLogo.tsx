/**
 * 0.4.6 (D-126) — a tool's logo on the making screens, the apps' own
 * icons read at run time from this Mac (D-38, `AppIcon`), never bundled:
 *
 * - a desktop app shows its own installed icon (a profile's own app, or
 *   the tool's app from the catalog: `host_app`);
 * - a CLI shows its vendor's desktop app icon (the same catalog field)
 *   with a small terminal badge, at 24 px and above;
 * - with no icon on this Mac, a filled neutral squircle: the tool's mark
 *   where its owner allows it (`ToolMark`'s registry), else its letter.
 *   OpenCode, Kimi and Grok keep the letter until their owners' rules are
 *   logged in docs/research/brand-marks.md (the registry says so; no
 *   branch on a tool here).
 *
 * Sizes: 24 in rows, 32 on cards, 40 on choices. Decorative: the tool's
 * name is beside it in words (`alt` names it where it isn't).
 * `LogoTile` is the same squircle with an icon in it, for what has no
 * logo (Everyone, This Mac, No lead).
 */
import { useMemo, type JSX, type SVGProps } from "react";
import { AppIconTile, macRadius } from "./AppIcon";
import { drawableMark, markLetters } from "./ToolMark";
import { IconTerminal } from "./icons";
import { toolSpec } from "../lib/tools";

export type ToolLogoSize = 24 | 32 | 40;
export type ToolLogoKind = "desktop" | "cli";

/** The filled neutral squircle every logo without an icon stands in. */
const SQUIRCLE = "flex size-full items-center justify-center bg-surface-3 text-ink inset-ring-1 inset-ring-hairline";

/** A tool's mark (where allowed) or its letter, in the squircle. */
function NoIcon({ tool, name, size }: { tool: string; name: string; size: number }): JSX.Element {
  const mark = drawableMark(tool);
  return (
    <span aria-hidden="true" className={SQUIRCLE} style={{ borderRadius: macRadius(size) }} data-mark={mark ? mark.id : "letter"}>
      {mark ? (
        <svg width={Math.round(size * 0.56)} height={Math.round(size * 0.56)} viewBox="0 0 24 24" fill="currentColor" focusable="false">
          <path d={mark.path} />
        </svg>
      ) : (
        <span className="font-semibold leading-none tracking-tight before:content-[attr(data-letters)]" data-letters={markLetters(name)} style={{ fontSize: Math.max(12, Math.round(size * 0.42)) }} />
      )}
    </span>
  );
}

/** The terminal badge at a CLI logo's corner. */
function TerminalBadge({ size }: { size: number }): JSX.Element {
  const side = Math.max(14, Math.round(size * 0.46));
  const glyph = Math.max(10, Math.round(size * 0.32));
  return (
    <span
      aria-hidden="true"
      className="absolute -bottom-[3px] -right-[3px] flex items-center justify-center rounded-full bg-surface-3 text-ink shadow-[0_0_0_2px_var(--color-surface)]"
      style={{ width: side, height: side }}
      data-testid="tool-logo-badge"
    >
      <IconTerminal width={glyph} height={glyph} strokeWidth={2.4} />
    </span>
  );
}

export function ToolLogo({
  tool,
  kind = "desktop",
  appId,
  appPid = null,
  name,
  size = 32,
  alt = "",
  className = "",
}: {
  /** The tool's catalog id. */
  tool: string;
  /** A desktop app, or a CLI (its vendor's app icon with a terminal badge). */
  kind?: ToolLogoKind;
  /** A particular app's own icon (a place's or a profile's app id); the
   * tool's app from the catalog when left out. */
  appId?: string | null;
  appPid?: number | null;
  /** For the tooltip and the letter; the tool's name when left out. */
  name?: string;
  size?: ToolLogoSize;
  /** Words for it, only where no name is beside it. */
  alt?: string;
  className?: string;
}): JSX.Element {
  const spec = toolSpec(tool);
  const shown = name ?? spec.display_name;
  const iconOf = appId === undefined ? (spec.host_app ?? null) : appId;
  const fallback = useMemo(() => <NoIcon tool={tool} name={shown} size={size} />, [tool, shown, size]);
  return (
    <span className={`relative inline-flex shrink-0 ${className}`} style={{ width: size, height: size }} data-testid="tool-logo" data-tool={tool} data-kind={kind}>
      <AppIconTile appId={iconOf} appPid={appPid} name={shown} size={size} alt={alt} fallback={fallback} />
      {kind === "cli" && size >= 24 && <TerminalBadge size={size} />}
    </span>
  );
}

/** The logo squircle with an icon in it, for what has no logo of its own;
 * `on` takes the accent (a chosen card's, an active step's). */
export function LogoTile({ icon: Icon, size = 32, on = false }: { icon: (props: SVGProps<SVGSVGElement>) => JSX.Element; size?: ToolLogoSize; on?: boolean }): JSX.Element {
  const glyph = Math.round(size * 0.46);
  return (
    <span aria-hidden="true" className={"flex shrink-0 items-center justify-center " + (on ? "bg-accent-soft text-accent-hi" : "bg-surface-3 text-ink-2")} style={{ width: size, height: size, borderRadius: macRadius(size) }} data-testid="logo-tile">
      <Icon width={glyph} height={glyph} />
    </span>
  );
}
