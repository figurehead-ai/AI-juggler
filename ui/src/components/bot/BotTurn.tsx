/**
 * D-136, AJ-E3805 — one exchange: the question (with the scope it rode
 * on: "About api"), then what Bot found. The lead is code's, from each
 * row's own state; every answer says what it looked at; "I don't know"
 * says why and shows the nearest record or screen, labelled, never a
 * nearby answer. A Beta written answer sits above the numbered rows.
 */
import { useState, type JSX, type SVGProps } from "react";
import { BotRows } from "./BotRows";
import { BotWrittenCard } from "./BotWritten";
import { BotMemoryEcho, BotUsedMemories } from "./BotMemoryParts";
import { Marked } from "./Marked";
import { useBotNav } from "./bot-nav";
import { IconArrowRight, IconBot, IconCheckCircle, IconGauge, IconMessage } from "./bot-icons";
import { IconTerminal } from "../icons";
import { LogoTile } from "../ToolLogo";
import { FoldChevron } from "../controls";
import { HELP, OUT_OF_SCOPE, SHORTCUTS, WORDS, lead, lookedWords, nearestWords, unknownWords, type Piece } from "../../lib/bot-words";
import { clock, dayWords } from "../../lib/time-words";
import { ask, openBot, widenScope } from "../../lib/bot-store";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";
import type { BotShortcut } from "../../types/generated/BotShortcut";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";

const SHORTCUT_ICON: Record<BotShortcut, (p: SVGProps<SVGSVGElement>) => JSX.Element> = {
  needs_me: IconMessage,
  finished_today: IconCheckCircle,
  runway: IconGauge,
  running: IconTerminal,
  usage: IconGauge,
};

function Lead({ pieces }: { pieces: readonly Piece[] }): JSX.Element {
  return (
    <p className="text-body leading-normal text-ink-2" data-testid="bot-lead">
      {pieces.map((p, i) => (p.strong ? <b key={i} className="font-semibold text-ink">{p.text}</b> : <span key={i}>{p.text}</span>))}
    </p>
  );
}

export function Shortcut({ shortcut }: { shortcut: BotShortcut }): JSX.Element {
  const nav = useBotNav();
  const words = SHORTCUTS[shortcut];
  return (
    <button type="button" className="flex w-full items-center gap-3 rounded-xl bg-surface-2 px-3 py-2.5 text-left outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => nav.openShortcut(shortcut)} data-testid={`bot-shortcut-${shortcut}`}>
      <LogoTile icon={SHORTCUT_ICON[shortcut]} size={24} />
      <span className="flex min-w-0 grow flex-col">
        <span className="text-body font-medium text-ink">{words.title}</span>
        <span className="text-meta text-muted">{words.where}</span>
      </span>
      <IconArrowRight width={15} height={15} aria-hidden="true" className="text-faint" />
    </button>
  );
}

function Question({ turn, find, current }: { turn: BotTurnInfo; find?: string; current: boolean }): JSX.Element {
  return (
    <div className="max-w-[86%] self-end rounded-[14px_14px_4px_14px] bg-surface-3 px-3 py-2 text-body leading-snug text-ink" data-testid="bot-question">
      {turn.scope && (
        <span className="mb-1 flex items-center justify-end gap-1.5 text-label text-muted">
          <IconBot width={12} height={12} aria-hidden="true" />
          About <b className="font-semibold text-ink-2">{turn.scope.name}</b>
        </span>
      )}
      {turn.redacted ? <span className="text-muted italic">Forgotten</span> : <Marked text={turn.question} words={find} current={current} />}
      <span className="mt-0.5 block text-right text-label text-muted">{clock(turn.at_ms)}</span>
    </div>
  );
}

function Looked({ turn, now }: { turn: BotTurnInfo; now: number }): JSX.Element {
  const [open, setOpen] = useState(false);
  const scoped = turn.looked.reach === "scope";
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-meta text-muted" data-testid="bot-looked">
        <button type="button" className="inline-flex items-center gap-1.5 rounded-md outline-none focus-visible:ring-2 focus-visible:ring-accent/60" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
          <FoldChevron open={open} />
          <span className="font-medium text-ink-2">{WORDS.looked}</span>
        </button>
        <span>· {lookedWords(turn.looked, turn.at_ms, now)}</span>
        {scoped && (
          <button type="button" className="ml-1 rounded-md font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => { widenScope(); if (turn.question) void ask(turn.question); }} data-testid="bot-widen-looked">
            {WORDS.widen}
          </button>
        )}
      </div>
      {open && <BotUsedMemories turn={turn} />}
    </div>
  );
}

/** Ways to narrow or widen a look-up, each one more question. */
function Refine({ turn }: { turn: BotTurnInfo }): JSX.Element | null {
  if (turn.scope || turn.rows.length < 2 || turn.outcome !== "rows") return null;
  const projects = new Map(turn.rows.filter((r) => r.project && r.project_id).map((r) => [r.project_id!, r.project!]));
  const chips: { label: string; run: () => void }[] = [];
  if (projects.size >= 1) {
    const [id, name] = [...projects.entries()][0]!;
    chips.push({ label: `Only ${name}`, run: () => { openBot({ kind: "project", id, name, sessions: [] }); void ask(turn.question); } });
  }
  if (!/today|morning|heute/i.test(turn.question)) chips.push({ label: "Since this morning", run: () => void ask(`${turn.question.replace(/\?$/, "")} since this morning?`) });
  if (chips.length === 0) return null;
  return (
    <div className="-mt-1 flex flex-wrap gap-1.5">
      {chips.map((chip) => (
        <button key={chip.label} type="button" className="inline-flex h-7 items-center rounded-full px-2.5 text-meta text-ink-2 inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={chip.run}>
          {chip.label}
        </button>
      ))}
    </div>
  );
}

function Unknown({ turn, now, find, onWatch }: { turn: BotTurnInfo; now: number; find?: string; onWatch?: (row: BotRowInfo) => void }): JSX.Element {
  const nav = useBotNav();
  const words = unknownWords(turn.unknown ?? "nothing_found");
  const nearest = turn.rows[0];
  return (
    <div className="flex flex-col gap-2">
      <p className="text-body leading-normal text-ink-2" data-testid="bot-lead">
        <b className="font-semibold text-ink">{words.strong}</b>
        {words.rest}
      </p>
      {nearest && turn.unknown && turn.unknown !== "nothing_found" ? (
        <>
          <span className="text-meta text-muted">
            {nearestWords(turn.unknown, nearest).lead}
            <b className="font-medium text-ink-2">{nearestWords(turn.unknown, nearest).name}</b>
            {nearestWords(turn.unknown, nearest).tail}
          </span>
          <BotRows rows={[nearest]} now={now} find={find} onWatch={onWatch} label="The nearest record" />
          {nearest.kind === "session" && !nearest.gone && (
            <span className="text-meta text-muted">
              Its agent may know:{" "}
              <button type="button" className="inline-flex items-center gap-1 rounded-md font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => nav.openRow(nearest)}>
                open “{nearest.title}” and ask it
                <IconArrowRight width={12} height={12} aria-hidden="true" />
              </button>
            </span>
          )}
        </>
      ) : (
        turn.shortcut && <Shortcut shortcut={turn.shortcut} />
      )}
      <Looked turn={turn} now={now} />
    </div>
  );
}

/** The names the user gave the rows (the memories this turn used), by `kind:id`. */
function aliasesOf(turn: BotTurnInfo): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of turn.used) {
    const [slot, kind, id] = m.key.split(":");
    if (slot === "alias" && kind && id) out.set(`${kind}:${id}`, m.text);
  }
  return out;
}

/** The answer part of an exchange. */
function Answer({ turn, now, latest, find, onWatch }: { turn: BotTurnInfo; now: number; latest: boolean; find?: string; onWatch?: (row: BotRowInfo) => void }): JSX.Element {
  const [recordsOnly, setRecordsOnly] = useState(false);
  if (turn.outcome === "help" || turn.outcome === "out_of_scope") {
    return <p className="text-body leading-normal text-ink-2" data-testid="bot-lead">{turn.outcome === "help" ? HELP : OUT_OF_SCOPE}</p>;
  }
  // A forgotten memory leaves nothing to show (the Bot review's H1): no
  // echo, no receipt, only that it was forgotten.
  if (turn.redacted && (turn.outcome === "remembered" || turn.outcome === "refused")) {
    return <p className="text-meta text-muted italic" data-testid="bot-forgotten">Forgotten.</p>;
  }
  if (turn.outcome === "remembered" || turn.outcome === "refused") return <BotMemoryEcho turn={turn} />;
  if (turn.outcome === "dont_know") return <Unknown turn={turn} now={now} find={find} onWatch={onWatch} />;
  const before = dayWords(turn.at_ms, now) !== "Today";
  const written = turn.written && !recordsOnly ? turn.written : null;
  return (
    <div className="flex flex-col gap-2">
      {written ? (
        <BotWrittenCard written={written} rows={turn.rows.length} turnId={turn.id} flagged={turn.flagged} onRecordsOnly={() => setRecordsOnly(true)} />
      ) : before && turn.rows.length > 0 && !latest ? (
        <span className="text-meta text-muted">{WORDS.askedBefore(turn.at_ms, now)}</span>
      ) : (
        <Lead pieces={lead(turn.rows, turn.lang, turn.scope)} />
      )}
      {turn.rows.length > 0 && <BotRows rows={turn.rows} numbered={written !== null} now={now} find={find} onWatch={onWatch} label="What Bot found" aliases={aliasesOf(turn)} />}
      {turn.total > turn.rows.length && <span className="pl-1 text-meta text-muted">{WORDS.more(turn.total - turn.rows.length)}</span>}
      <Looked turn={turn} now={now} />
      {latest && <Refine turn={turn} />}
    </div>
  );
}

/** One exchange. */
export function BotTurn({ turn, now, latest, find, current, onWatch }: { turn: BotTurnInfo; now: number; latest: boolean; find?: string; current?: boolean; onWatch?: (row: BotRowInfo) => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-[18px]" data-testid="bot-turn" data-turn={turn.id} data-outcome={turn.outcome}>
      <Question turn={turn} find={find} current={current ?? false} />
      <Answer turn={turn} now={now} latest={latest} find={find} onWatch={onWatch} />
    </div>
  );
}
