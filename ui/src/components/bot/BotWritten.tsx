/**
 * AJ-E3806, D-131 — a Beta written answer: labelled "Written by AI ·
 * Beta", each sentence with its numbered citations, "Checked against the
 * records below", and "How this was made" (the model, on this Mac, the
 * check, Beta's honesty line, This is wrong, Show records only). It is
 * shown only because every name, number and state in it was found in the
 * records it cites (the backend's check).
 */
import { useState, type JSX } from "react";
import { showToast } from "../Toast";
import { IconCopy } from "../icons-actions";
import { IconCheckCircle, IconChip, IconFlag, IconSparkle } from "./bot-icons";
import { IconShield } from "../icons";
import { FoldChevron } from "../controls";
import { WORDS } from "../../lib/bot-words";
import { botWrong } from "../../lib/ipc/bot";
import type { BotWritten } from "../../types/generated/BotWritten";

export function BetaPill({ children }: { children: string }): JSX.Element {
  return (
    <span className="inline-flex h-[22px] items-center gap-1.5 rounded-full bg-accent-soft px-2.5 text-label font-medium text-accent-hi">
      <IconSparkle width={12} height={12} aria-hidden="true" />
      {children}
    </span>
  );
}

export function BotWrittenCard({ written, rows, turnId, flagged, onRecordsOnly }: { written: BotWritten; rows: number; turnId: string; flagged: boolean; onRecordsOnly: () => void }): JSX.Element {
  const [open, setOpen] = useState(false);
  const [said, setSaid] = useState(flagged);
  const text = written.parts.map((p) => p.text).join(" ");
  const wrong = async () => {
    await botWrong(turnId);
    setSaid(true);
  };
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      showToast({ id: "bot-copied", message: "Copied the answer." });
    } catch {
      showToast({ id: "bot-copied", message: "Couldn’t copy the answer." });
    }
  };
  return (
    <section className="flex flex-col gap-2.5 rounded-[14px] bg-surface px-3.5 py-3 inset-ring-1 inset-ring-accent/35 [[data-theme=dark]_&]:bg-surface-2" aria-label={WORDS.writtenLabel} data-testid="bot-written">
      <div className="flex items-center gap-2">
        <BetaPill>{WORDS.writtenLabel}</BetaPill>
        <span className="grow" />
        <button type="button" className="grid size-[30px] place-items-center rounded-lg text-muted outline-none hover:bg-surface-3 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60" title={WORDS.copy} aria-label={WORDS.copy} onClick={() => void copy()}>
          <IconCopy width={15} height={15} aria-hidden="true" />
        </button>
      </div>
      <p className="text-body leading-relaxed text-ink">
        {written.parts.map((part, i) => (
          <span key={i}>
            {i > 0 ? " " : ""}
            {part.text}
            {part.cites.map((c) => (
              <span key={c} className="mx-px inline-grid h-[18px] min-w-[18px] place-items-center rounded-md bg-accent-soft px-1 align-[1px] text-label font-semibold text-accent-hi tabular-nums" aria-label={`record ${c}`}>
                {c}
              </span>
            ))}
          </span>
        ))}
      </p>
      <span className="flex items-center gap-1.5 text-meta text-ink-2">
        <IconCheckCircle width={15} height={15} aria-hidden="true" className="text-success" />
        {WORDS.checked(rows)}
      </span>
      <button type="button" className="inline-flex items-center gap-1.5 self-start rounded-md text-meta font-medium text-ink-2 outline-none focus-visible:ring-2 focus-visible:ring-accent/60" aria-expanded={open} onClick={() => setOpen((o) => !o)} data-testid="bot-how-made">
        <FoldChevron open={open} />
        {WORDS.howMade}
      </button>
      {open && (
        <div className="flex flex-col rounded-xl bg-surface-2 px-1 py-1.5 [[data-theme=dark]_&]:bg-surface-3" data-testid="bot-made">
          <div className="flex items-start gap-2.5 px-2 py-1.5 text-meta leading-snug text-ink-2">
            <IconChip width={15} height={15} aria-hidden="true" className="mt-px shrink-0 text-faint" />
            <span>{WORDS.madeBy(written.model_name)}</span>
          </div>
          <div className="flex items-start gap-2.5 px-2 py-1.5 text-meta leading-snug text-ink-2">
            <IconShield width={15} height={15} aria-hidden="true" className="mt-px shrink-0 text-faint" />
            <span>{WORDS.madeCheck(rows)}</span>
          </div>
          <div className="flex items-start gap-2.5 px-2 py-1.5 text-meta leading-snug text-ink-2">
            <IconFlag width={15} height={15} aria-hidden="true" className="mt-px shrink-0 text-faint" />
            <span>{WORDS.madeBeta}</span>
          </div>
          <div className="flex items-center gap-2.5 px-2 pt-1.5 pb-0.5">
            <button type="button" disabled={said} className="inline-flex h-7 items-center gap-1.5 rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-60" onClick={() => void wrong()} data-testid="bot-wrong">
              <IconFlag width={13} height={13} aria-hidden="true" />
              {said ? WORDS.wrongSaid : WORDS.wrong}
            </button>
            <span className="grow" />
            <button type="button" className="rounded-md text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={onRecordsOnly} data-testid="bot-records-only">
              {WORDS.recordsOnly}
            </button>
          </div>
          <span className="px-2 pb-1 text-label text-faint">{WORDS.wrongKept}</span>
        </div>
      )}
    </section>
  );
}
