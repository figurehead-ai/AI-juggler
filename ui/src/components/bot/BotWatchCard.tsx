/**
 * D-136 point 5, AJ-E3815 — acting needs a confirm card. "Watch…" on a
 * found row builds the watch by code from the row's id (watch drafting
 * from words is "not yet as AI", v5-final): the target named in full,
 * what it can be watched for (only what its App reports, D-127), the echo
 * sentence the code writes, quiet hours in one line, and Watch / Change…
 * / Cancel. Nothing is written until Watch is clicked (AJ-E3814's
 * create command); Bot acts only through a card like this.
 */
import { useEffect, useMemo, useRef, useState, type JSX, type SVGProps } from "react";
import { IconClose } from "../icons-controls";
import { IconShield } from "../icons";
import { IconCheckCircle, IconEye, IconFail, IconHourglass, IconMessage, IconMoon } from "./bot-icons";
import { RowMarkFor } from "./BotRowMark";
import { STATE_WORDS, WORDS, rowPlace } from "../../lib/bot-words";
import { watchesCatalogue, watchesCreate, watchesPreview, type CatalogueEntry, type WatchDraftInfo } from "../../lib/ipc/watches";
import { notificationPrefsGet } from "../../lib/ipc/settings";
import { errorText } from "../../lib/bridge";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";
import type { LifetimeChoice } from "../../types/generated/LifetimeChoice";

const CONDITION_ICON: Record<string, (p: SVGProps<SVGSVGElement>) => JSX.Element> = {
  finishes: IconCheckCircle,
  asks_you: IconMessage,
  stuck: IconHourglass,
  fails: IconFail,
};

/** The four the card offers first (the closed set's most asked). */
const FIRST = ["finishes", "asks_you", "stuck", "fails"];

const LIFETIMES: { choice: LifetimeChoice; label: string }[] = [
  { choice: { kind: "once" }, label: "Once" },
  { choice: { kind: "until_ends" }, label: "Until it ends" },
  { choice: { kind: "today" }, label: "Today" },
  { choice: { kind: "always" }, label: "Always" },
];

function hour(h: number): string {
  return `${String(h).padStart(2, "0")}:00`;
}

export function BotWatchCard({ row, turnId, onClose }: { row: BotRowInfo; turnId: string; onClose: () => void }): JSX.Element {
  const card = useRef<HTMLElement | null>(null);
  const [offered, setOffered] = useState<CatalogueEntry[] | null>(null);
  const [codes, setCodes] = useState<string[]>([]);
  const [lifetime, setLifetime] = useState<LifetimeChoice>({ kind: "once" });
  const [changing, setChanging] = useState(false);
  const [words, setWords] = useState<string>("");
  const [problem, setProblem] = useState<string | null>(null);
  const [quiet, setQuiet] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (offered) card.current?.scrollIntoView?.({ block: "nearest" });
  }, [offered]);
  useEffect(() => {
    void watchesCatalogue({ kind: "session", id: row.id })
      .then((entries) => {
        const shown = entries.filter((e) => FIRST.includes(e.code));
        setOffered(shown);
        setCodes(shown.some((e) => e.code === "finishes") ? ["finishes"] : shown.slice(0, 1).map((e) => e.code));
      })
      .catch((cause) => setError(errorText(cause)));
    void notificationPrefsGet()
      .then((prefs) => setQuiet(prefs.quiet_start === prefs.quiet_end ? null : `Between ${hour(prefs.quiet_start)} and ${hour(prefs.quiet_end)} it waits till morning.`))
      .catch(() => setQuiet(null));
  }, [row.id]);
  const draft: WatchDraftInfo = useMemo(
    () => ({ targets: [{ kind: "session", id: row.id }], matching: "each", conditions: codes.map((code) => ({ code })), lifetime, evenInQuietHours: false, chatTurn: turnId }),
    [row.id, codes, lifetime, turnId],
  );
  useEffect(() => {
    let live = true;
    if (codes.length > 0) {
      void watchesPreview(draft)
        .then((preview) => {
          if (!live) return;
          setWords(preview.words);
          setProblem(preview.problem);
        })
        .catch((cause) => live && setError(errorText(cause)));
    }
    return () => {
      live = false;
    };
  }, [draft, codes.length]);
  const confirm = async () => {
    try {
      const made = await watchesCreate(draft);
      setDone(made.words);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  if (done) {
    return (
      <output className="flex items-center gap-2.5 rounded-[14px] bg-surface px-3.5 py-3 text-body text-ink-2 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="bot-watch-done">
        <IconEye width={16} height={16} aria-hidden="true" className="shrink-0 text-accent-hi" />
        {WORDS.watchSet(done)}
      </output>
    );
  }
  const toggle = (code: string) => setCodes((now) => (now.includes(code) ? now.filter((c) => c !== code) : [...now, code]));
  return (
    <section ref={card} aria-label="Confirm a new watch" className="flex flex-col gap-3 rounded-[14px] bg-[color-mix(in_srgb,var(--color-accent)_6%,var(--color-surface))] p-3.5 inset-ring-[1.5px] inset-ring-accent/55 [[data-theme=dark]_&]:bg-[color-mix(in_srgb,var(--color-accent)_9%,var(--color-surface))]" data-testid="bot-watch-card">
      <div className="flex items-center gap-2">
        <span className="grow text-label font-semibold tracking-[.07em] text-accent-hi uppercase">{WORDS.confirmEyebrow}</span>
        <button type="button" className="grid size-6 place-items-center rounded-lg text-muted outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" title={WORDS.cancel} aria-label={WORDS.cancel} onClick={onClose}>
          <IconClose width={14} height={14} aria-hidden="true" />
        </button>
      </div>
      <div className="flex items-center gap-2.5">
        <RowMarkFor row={row} />
        <div className="flex min-w-0 flex-col">
          <span className="text-body font-medium text-ink">{row.title}</span>
          <span className="text-meta text-muted">{[rowPlace(row), row.state ? STATE_WORDS[row.state] : null].filter(Boolean).join(" · ")}</span>
        </div>
      </div>
      <fieldset className="flex flex-col gap-1.5">
        <legend className="mb-1.5 text-meta text-ink-2">{WORDS.tellMeWhen}</legend>
        <div className="grid grid-cols-2 gap-1.5">
          {(offered ?? []).map((entry) => {
            const on = codes.includes(entry.code);
            const Icon = CONDITION_ICON[entry.code] ?? IconCheckCircle;
            return (
              <button key={entry.code} type="button" aria-pressed={on} className={"inline-flex h-8 items-center gap-1.5 rounded-[10px] px-2.5 text-meta outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (on ? "bg-[color-mix(in_srgb,var(--color-accent)_9%,var(--color-surface))] font-medium text-ink inset-ring-[1.5px] inset-ring-accent" : "text-ink-2 inset-ring-1 inset-ring-hairline hover:bg-surface-2")} onClick={() => toggle(entry.code)} data-testid={`bot-watch-${entry.code}`}>
                <Icon width={14} height={14} aria-hidden={true} />
                {entry.label}
              </button>
            );
          })}
        </div>
      </fieldset>
      {changing && (
        <div className="flex flex-wrap gap-1.5" aria-label="For how long">
          {LIFETIMES.map(({ choice, label }) => (
            <button key={choice.kind} type="button" aria-pressed={lifetime.kind === choice.kind} className={"inline-flex h-7 items-center rounded-full px-2.5 text-meta outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (lifetime.kind === choice.kind ? "bg-selected font-semibold text-ink inset-ring-[1.5px] inset-ring-selected-ring" : "text-ink-2 inset-ring-1 inset-ring-hairline")} onClick={() => setLifetime(choice)}>
              {label}
            </button>
          ))}
        </div>
      )}
      <p className="rounded-[10px] bg-surface px-3 py-2.5 text-[15px] leading-normal text-ink-2 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" aria-live="polite" data-testid="bot-watch-echo">
        {(codes.length > 0 && words) || "Choose what to be told about."}
      </p>
      {problem && <p className="text-meta text-danger">{problem}</p>}
      {quiet && (
        <span className="flex items-center gap-2 text-meta text-muted">
          <IconMoon width={14} height={14} aria-hidden="true" />
          {quiet}
        </span>
      )}
      {error && <p className="text-meta text-danger" role="alert">{error}</p>}
      <div className="flex items-center gap-2">
        <button type="button" disabled={codes.length === 0 || problem !== null} className="inline-flex h-7 items-center gap-1.5 rounded-lg bg-accent-fill px-2.5 text-meta font-medium text-white shadow-glow-accent inset-ring-1 inset-ring-accent-hi/40 outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-50 disabled:shadow-none" onClick={() => void confirm()} data-testid="bot-watch-confirm">
          <IconEye width={14} height={14} aria-hidden="true" />
          {WORDS.watchButton}
        </button>
        <button type="button" aria-expanded={changing} className="inline-flex h-7 items-center rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => setChanging((c) => !c)}>
          {WORDS.change}
        </button>
        <button type="button" className="inline-flex h-7 items-center rounded-lg px-2.5 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={onClose} data-testid="bot-watch-cancel">
          {WORDS.cancel}
        </button>
      </div>
      <span className="flex items-center gap-1.5 text-label text-muted">
        <IconShield width={13} height={13} aria-hidden="true" />
        {WORDS.actsOnly}
      </span>
    </section>
  );
}
