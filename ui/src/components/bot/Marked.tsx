/**
 * D-136 — find in the conversation: the words marked where they appear,
 * like find in a page. Nothing is filtered, so the conversation stays
 * one stream.
 */
import type { JSX } from "react";

export function Marked({ text, words, current = false }: { text: string; words?: string; current?: boolean }): JSX.Element {
  const w = words?.trim();
  if (!w) return <>{text}</>;
  const lower = text.toLowerCase();
  const needle = w.toLowerCase();
  const parts: JSX.Element[] = [];
  let at = 0;
  let i = lower.indexOf(needle);
  while (i >= 0) {
    if (i > at) parts.push(<span key={`t${at}`}>{text.slice(at, i)}</span>);
    parts.push(
      <mark key={`m${i}`} className={"rounded-[3px] px-px " + (current ? "bg-warning text-canvas" : "bg-warning/35 text-ink")} data-testid="bot-mark">
        {text.slice(i, i + needle.length)}
      </mark>,
    );
    at = i + needle.length;
    i = lower.indexOf(needle, at);
  }
  if (at < text.length) parts.push(<span key={`t${at}`}>{text.slice(at)}</span>);
  return <>{parts}</>;
}
