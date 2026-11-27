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
import { IconCheckCircle, IconFail, IconHourglass, IconMessage } from "./bot-icons";
import { RowMarkFor } from "./BotRowMark";
import { ConfirmFoot, ConfirmNotes, ConfirmShell, EchoLine, WatchSet, useQuietLine } from "./BotConfirmParts";
import { STATE_WORDS, WORDS, rowPlace } from "../../lib/bot-words";
import { openWatches } from "../../lib/watches-open";
import { watchesCatalogue, watchesCreate, watchesPreview, type CatalogueEntry, type WatchDraftInfo } from "../../lib/ipc/watches";
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

/** The way to the full form. */
const WATCH_MORE = "More choices…";

/** A session that isn't running: what it is watched for happens only once
 * it's resumed (the 0.5 walk's N-09: "Finishes" read as if it could fire
 * now), and "Until it ends" means nothing for one that has. */
const NOT_RUNNING = new Set<BotRowInfo["state"]>(["finished", "done", "ended", "failed"]);
const RESUMED_NOTE = "It isn't running now: the watch tells you once it's resumed.";

export function BotWatchCard({ row, turnId, onClose }: { row: BotRowInfo; turnId: string; onClose: () => void }): JSX.Element {
  const over = NOT_RUNNING.has(row.state);
  const card = useRef<HTMLElement | null>(null);
  const [offered, setOffered] = useState<CatalogueEntry[] | null>(null);
  const [codes, setCodes] = useState<string[]>([]);
  const [lifetime, setLifetime] = useState<LifetimeChoice>({ kind: "once" });
  const [changing, setChanging] = useState(false);
  const [words, setWords] = useState<string>("");
  const [problem, setProblem] = useState<string | null>(null);
  const quiet = useQuietLine();
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
  if (done) return <WatchSet words={done} />;
  const toggle = (code: string) => setCodes((now) => (now.includes(code) ? now.filter((c) => c !== code) : [...now, code]));
  return (
    <ConfirmShell label="Confirm a new watch" testId="bot-watch-card" onClose={onClose} cardRef={(node) => (card.current = node)}>
      <div className="flex items-center gap-2.5">
        <RowMarkFor row={row} />
        <div className="flex min-w-0 flex-col">
          <span className="text-body font-medium text-ink">{row.title}</span>
          <span className="text-meta text-muted">{[rowPlace(row), row.state ? STATE_WORDS[row.state] : null].filter(Boolean).join(" · ")}</span>
        </div>
      </div>
      {over && (
        <p className="text-meta text-ink-2" data-testid="bot-watch-not-running">
          {RESUMED_NOTE}
        </p>
      )}
      <fieldset className="flex flex-col gap-1.5">
        <legend className="mb-1.5 text-meta text-ink-2">{WORDS.tellMeWhen}</legend>
        <div className="grid grid-cols-2 gap-1.5">
          {(offered ?? []).map((entry) => {
            const on = codes.includes(entry.code);
            const Icon = CONDITION_ICON[entry.code] ?? IconCheckCircle;
            return (
              <button key={entry.code} type="button" aria-pressed={on} className={"inline-flex h-8 items-center gap-1.5 rounded-[10px] px-2.5 text-meta outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (on ? "bg-selected font-medium text-ink" : "text-ink-2 inset-ring-1 inset-ring-hairline hover:bg-surface-2")} onClick={() => toggle(entry.code)} data-testid={`bot-watch-${entry.code}`}>
                <Icon width={14} height={14} aria-hidden={true} />
                {entry.label}
              </button>
            );
          })}
        </div>
      </fieldset>
      {changing && (
        <div className="flex flex-wrap gap-1.5" aria-label="For how long">
          {LIFETIMES.filter(({ choice }) => !(over && choice.kind === "until_ends")).map(({ choice, label }) => (
            <button key={choice.kind} type="button" aria-pressed={lifetime.kind === choice.kind} className={"inline-flex h-7 items-center rounded-full px-2.5 text-meta outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (lifetime.kind === choice.kind ? "bg-selected font-semibold text-ink" : "text-ink-2 inset-ring-1 inset-ring-hairline")} onClick={() => setLifetime(choice)}>
              {label}
            </button>
          ))}
          {/* AJ-E3816: every choice, in the Watches form, filled in (the confirm stays the user's click there). */}
          <button type="button" className="inline-flex h-7 items-center rounded-full px-2.5 text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => openWatches({ kind: "new", draft: { ...draft, chatTurn: turnId } })} data-testid="bot-watch-more">
            {WATCH_MORE}
          </button>
        </div>
      )}
      <EchoLine words={(codes.length > 0 && words) || "Choose what to be told about."} />
      <ConfirmNotes problem={problem} quiet={quiet} error={error} />
      <ConfirmFoot canWatch={codes.length > 0 && problem === null} onWatch={() => void confirm()} onChange={() => setChanging((c) => !c)} changeExpanded={changing} onCancel={onClose} />
    </ConfirmShell>
  );
}
