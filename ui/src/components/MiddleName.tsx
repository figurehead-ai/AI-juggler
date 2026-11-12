/**
 * D-100 (0.4.2 review): a name cut in the middle, keeping its start and
 * its end ("acme-east"). Profile names share a prefix ("acme-east",
 * "acme-west"), so an end cut ("acme-l…") hid the part that tells
 * two Apps apart. Whole whenever it fits; the full name is the caller's
 * title. The end gives way from its left (an `rtl` box around an
 * isolated `ltr` run, so the characters keep their order).
 */
import type { JSX } from "react";

/** How much of the start always shows. */
const HEAD = 4;

export function MiddleName({ name, className = "" }: { name: string; className?: string }): JSX.Element {
  if (name.length <= HEAD + 4) return <span className={"min-w-0 truncate " + className}>{name}</span>;
  return (
    <span className={"flex min-w-0 " + className} data-testid="middle-name">
      <span className="shrink-0 whitespace-pre">{name.slice(0, HEAD)}</span>
      <span dir="rtl" className="min-w-0 truncate">
        <bdi dir="ltr">{name.slice(HEAD)}</bdi>
      </span>
    </span>
  );
}
