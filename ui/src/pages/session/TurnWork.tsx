/**
 * 0.3.4 — a finished turn as the tools' own windows show it: its work
 * (notes written while it worked, its steps) folded into one row that
 * opens onto it, then its final reply (`LastMessages` draws the turn).
 */
import { useState, type JSX } from "react";
import type { LastTurnInfo, TurnBlockInfo } from "../../lib/ipc/conversations";
import { ageWords } from "../../lib/process-states";
import { Markdown } from "./Markdown";

/**
 * 0.3.4: a turn as the tools' own windows show it (the Claude app's Code
 * tab, ChatGPT's Codex, Kimi Code): its work (the words written between
 * steps, and the steps) folded into one row, then its final reply. A turn
 * that ends on a step has no reply yet: all of it is work.
 */
export function foldOf(turn: LastTurnInfo): { work: TurnBlockInfo[]; reply: TurnBlockInfo | null } {
  const last = turn.blocks[turn.blocks.length - 1];
  return last?.kind === "text" ? { work: turn.blocks.slice(0, -1), reply: last } : { work: turn.blocks, reply: null };
}

/** The work row's words: "Worked for 4 min · Ran 3 commands, edited a
 * file · 2 notes" (each part only when known). */
export function workWords(work: TurnBlockInfo[], tookMs: number | null): string {
  const lines = work.flatMap((block) => (block.kind === "steps" ? block.steps : []));
  const notes = work.filter((block) => block.kind === "text").length;
  const parts = [
    tookMs !== null && tookMs > 0 ? `Worked for ${ageWords(tookMs)}` : null,
    lines.length > 0 ? stepsSummary(lines) : null,
    notes > 0 ? `${notes} ${notes === 1 ? "note" : "notes"}` : null,
  ].filter(Boolean) as string[];
  const words = parts.join(" · ");
  return words ? words.charAt(0).toUpperCase() + words.slice(1) : "Worked";
}

export function Blocks({ blocks }: { blocks: TurnBlockInfo[] }): JSX.Element {
  return <>{blocks.map((block, index) => (block.kind === "steps" ? <Steps key={index} block={block} /> : <Markdown key={index} text={block.text} />))}</>;
}

/** A turn's work, folded: one row that opens onto it, as it was written. */
export function Work({ work, words }: { work: TurnBlockInfo[]; words: string }): JSX.Element {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex flex-col" data-testid="last-turn-work">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((now) => !now)}
        className="inline-flex w-fit max-w-full items-center gap-2 rounded-lg px-1 py-1 text-left text-meta font-medium text-muted outline-none transition-colors hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        data-testid="last-turn-work-toggle"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={"shrink-0 transition-transform " + (open ? "rotate-90" : "")}>
          <path d="M9 6l6 6-6 6" />
        </svg>
        <span className="truncate">{words}</span>
      </button>
      {open && (
        <div className="mt-2 flex flex-col gap-3 border-l border-hairline pl-4" data-testid="last-turn-work-blocks">
          <Blocks blocks={work} />
        </div>
      )}
    </div>
  );
}

type StepKind = "command" | "read" | "edit" | "search" | "agent" | "plan" | "web" | "other";

/** What kind of call a line is, from its words (the adapters' summary). */
export function stepKind(line: string): StepKind {
  if (line.startsWith("ran a subagent")) return "agent";
  if (line.startsWith('ran "')) return "command";
  if (/^(Read|Glob)\b/.test(line) || line.startsWith("listed ")) return "read";
  if (/^(Edit|MultiEdit|Write|NotebookEdit)\b/.test(line) || line.startsWith("edited ")) return "edit";
  if (line.startsWith("searched the web") || line.startsWith("read ")) return "web";
  if (line.startsWith("searched")) return "search";
  if (line === "updated the task list") return "plan";
  return "other";
}

/** A run of calls in one line: "Ran 2 commands, read 3 files". */
export function stepsSummary(lines: string[]): string {
  const count: Record<StepKind, number> = { command: 0, read: 0, edit: 0, search: 0, agent: 0, plan: 0, web: 0, other: 0 };
  for (const line of lines) {
    const kind = stepKind(line);
    // "edited a.ts, b.ts" is two files.
    count[kind] += kind === "edit" && line.startsWith("edited ") ? line.slice(7).split(", ").length : 1;
  }
  const n = (k: number, one: string, many: string) => `${k} ${k === 1 ? one : many}`;
  const parts = [
    count.command && `ran ${n(count.command, "command", "commands")}`,
    count.edit && `edited ${n(count.edit, "file", "files")}`,
    count.read && `read ${n(count.read, "file", "files")}`,
    count.search && `searched ${count.search === 1 ? "once" : `${count.search} times`}`,
    count.web && `looked on the web ${count.web === 1 ? "once" : `${count.web} times`}`,
    count.agent && `ran ${n(count.agent, "subagent", "subagents")}`,
    count.plan && "updated the task list",
    count.other && `used ${n(count.other, "tool", "tools")}`,
  ].filter(Boolean) as string[];
  const words = parts.join(", ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function Steps({ block }: { block: TurnBlockInfo }): JSX.Element {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex flex-col" data-testid="last-turn-steps">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((now) => !now)}
        className="inline-flex w-fit max-w-full items-center gap-2 rounded-lg border border-hairline bg-surface-2/60 px-2.5 py-1.5 text-left text-meta font-medium text-ink/85 outline-none transition-colors hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={"shrink-0 transition-transform " + (open ? "rotate-90" : "")}>
          <path d="M9 6l6 6-6 6" />
        </svg>
        <span className="truncate">{stepsSummary(block.steps)}</span>
      </button>
      {open && (
        <ul className="mt-2 flex flex-col gap-1 border-l border-hairline pl-4" data-testid="last-turn-step-lines">
          {block.hidden > 0 && <li className="text-label text-muted">{block.hidden === 1 ? "1 earlier step" : `${block.hidden} earlier steps`} not shown</li>}
          {block.steps.map((line, index) => (
            <li key={index} className="break-words font-mono text-meta leading-[1.5] text-ink/90">
              {line}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

