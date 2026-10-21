/**
 * AJ-E1526 — one mark for a tool or its company, wherever it is named.
 * The mark id comes from the tool's spec in the catalog (never a branch on
 * the tool's id). A third-party mark is drawn only when its owner allows
 * it (assets/third-party-marks); otherwise AI Juggler's own neutral letter
 * mark stands in. Always decorative: the tool's name is beside it.
 */
import type { CSSProperties, JSX, SVGProps } from "react";
import { MARKS } from "../assets/third-party-marks";
import { toolSpec } from "../lib/tools";
import { Chip, type ChipTone } from "./primitives";

/** The letter mark's letters: a two-word name's initials ("CC"), else its
 * first two letters ("Co"). */
export function markLetters(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  if (words.length >= 2) return `${words[0]![0]}${words[1]![0]}`.toUpperCase();
  const word = words[0] ?? "?";
  return `${word[0]!.toUpperCase()}${word.slice(1, 2).toLowerCase()}`;
}

/** The registry entry a tool (or its company) draws, if it may be drawn. */
export function drawableMark(tool: string, of: "tool" | "vendor" = "tool"): { id: string; path: string } | null {
  const spec = toolSpec(tool);
  const id = of === "vendor" ? (spec.vendor_mark ?? spec.mark) : spec.mark;
  const entry = id ? MARKS[id] : undefined;
  return id && entry?.allowed && entry.path ? { id, path: entry.path } : null;
}

export function ToolMark({
  tool,
  of = "tool",
  size = 16,
  className = "",
}: {
  tool: string;
  /** The tool's own mark, or its company's. */
  of?: "tool" | "vendor";
  size?: number;
  className?: string;
}): JSX.Element {
  const mark = drawableMark(tool, of);
  if (mark) {
    return (
      <svg
        aria-hidden="true"
        focusable="false"
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="currentColor"
        className={`shrink-0 ${className}`}
        data-mark={mark.id}
      >
        <path d={mark.path} />
      </svg>
    );
  }
  const spec = toolSpec(tool);
  const name = of === "vendor" && spec.vendor ? spec.vendor : spec.display_name;
  const style: CSSProperties = { width: size, height: size, fontSize: Math.max(7, Math.round(size * 0.46)) };
  return (
    <span
      aria-hidden="true"
      data-mark="letter"
      style={style}
      className={`inline-flex shrink-0 items-center justify-center rounded-[28%] border border-current font-semibold leading-none tracking-tight opacity-80 ${className}`}
    >
      {markLetters(name)}
    </span>
  );
}

/** A tool's chip: its mark beside its name. */
export function ToolChip({ tool, tone = "neutral" }: { tool: string; tone?: ChipTone }): JSX.Element {
  return (
    <Chip tone={tone}>
      <ToolMark tool={tool} size={11} />
      {toolSpec(tool).display_name}
    </Chip>
  );
}

/** An icon-shaped component for tool `tool` (for places that take an
 * `Icon` with width and height), cached so React keeps it stable. */
const iconCache = new Map<string, (props: SVGProps<SVGSVGElement>) => JSX.Element>();
export function toolMarkIcon(tool: string): (props: SVGProps<SVGSVGElement>) => JSX.Element {
  let icon = iconCache.get(tool);
  if (!icon) {
    icon = function ToolMarkIcon(props: SVGProps<SVGSVGElement>) {
      const size = Number(props.width ?? 16);
      return <ToolMark tool={tool} size={size} className={typeof props.className === "string" ? props.className : ""} />;
    };
    iconCache.set(tool, icon);
  }
  return icon;
}
