/**
 * ADR-0032 Amendment 3 (D-100) — an App's kind, by icon and word: a
 * window for a Desktop App, `>_` for a CLI App, so two Apps on one
 * Account ("Claude app · work", "Claude Code CLI · work") never look like
 * one App shown twice. Always beside the App's own icon, never over it
 * (the app-icons ruling); the word too where there's room.
 *
 * - `glyph`: the mark alone, 14 px, for the rail's tight rows (its words
 *   are the row's name and its tooltip);
 * - `chip`: the mark and its word ("Desktop", "CLI"), where a card, a
 *   page header or a picker names the App.
 */
import type { JSX } from "react";
import { IconTerminal } from "./icons";
import { IconWindow } from "./icons-places";
import { appKindWords, type AppKind } from "../lib/app-kind";

export type { AppKind } from "../lib/app-kind";

export function AppKindBadge({
  kind,
  variant = "glyph",
  className = "",
  wordClass = "",
  decorative = false,
}: {
  kind: AppKind | null | undefined;
  variant?: "glyph" | "chip";
  className?: string;
  /** Its kind is said in words beside it already ("Desktop · open now"):
   * the mark is hidden from assistive technology. */
  decorative?: boolean;
  /** A chip's word's own classes: on a narrow card it can give way to
   * the mark alone (`hidden @[15rem]:inline`), so the App's name is
   * never cut for it. */
  wordClass?: string;
}): JSX.Element | null {
  if (!kind) return null;
  const { word, name } = appKindWords(kind);
  const Glyph = kind === "cli" ? IconTerminal : IconWindow;
  // Its name is text ("CLI app"), read where the mark is; the glyph and
  // the visible word are pictures of it.
  const said = decorative ? null : <span className="sr-only">{name}</span>;
  if (variant === "chip") {
    return (
      <span
        title={name}
        aria-hidden={decorative ? "true" : undefined}
        data-testid="app-kind"
        data-kind={kind}
        className={"inline-flex h-[18px] shrink-0 items-center gap-1 whitespace-nowrap rounded border border-hairline bg-surface-2 px-1 text-label font-medium leading-none text-muted " + className}
      >
        <Glyph width={11} height={11} strokeWidth={2} aria-hidden="true" />
        <span aria-hidden="true" className={wordClass} data-testid="app-kind-word">
          {word}
        </span>
        {said}
      </span>
    );
  }
  return (
    <span
      title={name}
      aria-hidden={decorative ? "true" : undefined}
      data-testid="app-kind"
      data-kind={kind}
      className={"inline-flex size-[14px] shrink-0 items-center justify-center rounded-[3px] border border-hairline bg-surface-2 text-muted " + className}
    >
      <Glyph width={10} height={10} strokeWidth={2.25} aria-hidden="true" />
      {said}
    </span>
  );
}
