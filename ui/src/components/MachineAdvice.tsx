/**
 * E14 / AJ-E1407 — "Has this Mac got room?" in the New session sheet,
 * beside the account's own verdict ("Has this account got room?"). Two
 * questions, two answers; never one merged colour. It warns and explains;
 * it never blocks Start.
 */
import type { JSX } from "react";
import { Chip } from "./primitives";
import { bytesWords, reasonSentence, type MachineAdviceInfo } from "../lib/machine";

const GLYPH: Record<string, { glyph: string; tone: string }> = {
  ok: { glyph: "●", tone: "text-muted" },
  tight: { glyph: "▲", tone: "text-warning" },
  no: { glyph: "■", tone: "text-danger" },
};

/** What the projection is based on, in words. */
export function basisWords(advice: MachineAdviceInfo): string {
  return advice.based_on > 0
    ? `about ${bytesWords(advice.projected_bytes)} a session, from your last ${advice.based_on}`
    : `about ${bytesWords(advice.projected_bytes)} a session — not measured yet`;
}

export function MachineAdviceLine({
  advice,
  nameOf,
}: {
  advice: MachineAdviceInfo | undefined;
  /** A running session's name on screen. */
  nameOf: (sessionId: string) => string | null;
}): JSX.Element | null {
  if (!advice) return null;
  const look = GLYPH[advice.verdict] ?? GLYPH.ok!;
  const relief =
    advice.relief_session_id && advice.relief_bytes
      ? `Closing ${nameOf(advice.relief_session_id) ?? "your heaviest session"} frees about ${bytesWords(advice.relief_bytes)}.`
      : null;
  return (
    <div className="flex flex-col gap-0.5 text-xs" data-testid="machine-advice" data-verdict={advice.verdict} role="status">
      <span className="flex items-center gap-1.5 font-medium text-ink">
        <span aria-hidden="true" className={`text-label ${look.tone}`}>{look.glyph}</span>
        {advice.words}
      </span>
      <span className="text-muted">
        {advice.verdict === "ok" ? basisWords(advice) : `${reasonSentence(advice.reasons)}.`}
        {/* A Tight or Not-now verdict on the 1 GB default says so too. */}
        {advice.verdict !== "ok" && advice.based_on === 0 && ` Assumes ${basisWords(advice)}.`}
        {advice.verdict !== "ok" && ` ${relief ?? ""} You can still start it.`}
      </span>
    </div>
  );
}

/** A launch-settings row's machine note: only when it isn't OK. */
export function MachineChip({ advice }: { advice: MachineAdviceInfo | undefined }): JSX.Element | null {
  if (!advice || advice.verdict === "ok") return null;
  return (
    <Chip tone="neutral">
      <span
        data-testid="machine-chip"
        title={`${advice.words}. ${reasonSentence(advice.reasons)}.${advice.based_on === 0 ? ` Assumes ${basisWords(advice)}.` : ""}`}
      >
        This Mac: {advice.verdict === "no" ? "not now" : "tight"}
      </span>
    </Chip>
  );
}
