/**
 * AJ-E2027 (ADR-0039 §4, §11.1) — Bot's "remind me at…": code read the
 * request's time and words into a draft (no model picked anything); the
 * turn shows it as a card the user checks: the words, what it is about,
 * when. Change time offers the same times as Home's Remind me; only the
 * user's Set reminder click sets it, and that click primes the
 * Notifications prompt first (PAT-OSP, AJ-E1547's gate), never the
 * reminder's own time. Set, it lives in the Reminders list on the
 * Watches page.
 */
import { useState, type JSX, type MouseEvent } from "react";
import { Button } from "../primitives";
import { MenuItem, MenuLine, RowMenu, menuPoint, type MenuAt } from "../RowMenu";
import { IconClockFace } from "../watches/watch-icons";
import { REMIND_CHOICES } from "../../lib/feed-watch";
import { osPromptRequest } from "../../lib/ipc/os-prompts";
import { scheduleRemind } from "../../lib/ipc/schedules";
import { errorText } from "../../lib/bridge";
import { whenWords } from "../../lib/time-words";
import { openWatches } from "../../lib/watches-open";
import type { ReminderDraftInfo } from "../../types/generated/ReminderDraftInfo";

export const REMIND_DRAFT = {
  set: "Set reminder",
  change: "Change time",
  pick: "Pick a time",
  setFor: (when: string) => `Reminder set for ${when}.`,
  see: "See reminders",
  failed: (why: string) => `The reminder couldn't be set: ${why}`,
} as const;

export function BotReminderDraft({ draft, line }: { draft: ReminderDraftInfo; line: string }): JSX.Element {
  const [at, setAt] = useState<number | null>(draft.at_ms);
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const [state, setState] = useState<{ kind: "draft" } | { kind: "busy" } | { kind: "set"; at: number } | { kind: "failed"; why: string }>({ kind: "draft" });
  const [opened] = useState(() => Date.now());
  const set = async () => {
    if (at === null) return;
    setState({ kind: "busy" });
    // PAT-OSP: the Notifications prompt is primed at this click.
    await osPromptRequest("notifications").catch(() => undefined);
    try {
      const made = await scheduleRemind({ at_ms: at, title: draft.title, about: draft.about ?? null, chat_turn: draft.chat_turn ?? null });
      setState({ kind: "set", at: made.next_due_ms ?? at });
    } catch (error: unknown) {
      setState({ kind: "failed", why: errorText(error) });
    }
  };
  const done = state.kind === "set";
  return (
    <div className="flex flex-col items-start gap-2" data-testid="bot-reminder-draft">
      <p className="flex items-center gap-2 text-body leading-normal text-ink-2">
        <IconClockFace width={16} height={16} aria-hidden="true" className="shrink-0 text-accent-hi" />
        {line}
      </p>
      <div className="flex w-full max-w-[460px] items-center gap-3 rounded-card bg-surface-2 px-3 py-2.5 inset-ring-1 inset-ring-hairline/75">
        <span aria-hidden="true" className="flex size-9 shrink-0 items-center justify-center rounded-[10px] bg-accent-soft text-accent-hi">
          <IconClockFace width={18} height={18} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-body font-medium text-ink" data-testid="bot-reminder-title">
            {draft.title}
          </span>
          <span className={"text-meta " + (at === null ? "text-muted" : "font-medium text-accent-hi")} data-testid="bot-reminder-when">
            {at === null ? REMIND_DRAFT.pick : whenWords(at, opened)}
          </span>
        </span>
      </div>
      {done ? (
        <p className="flex flex-wrap items-center gap-2 text-meta text-ink-2" data-testid="bot-reminder-set">
          {REMIND_DRAFT.setFor(whenWords(state.at, opened))}
          <button type="button" className="font-medium text-accent-hi underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => openWatches({ kind: "list" })} data-testid="bot-reminder-see">
            {REMIND_DRAFT.see}
          </button>
        </p>
      ) : (
        <div className="flex items-center gap-2">
          <Button variant="primary" size="sm" disabled={at === null || state.kind === "busy"} onClick={() => void set()} data-testid="bot-reminder-set-button">
            {REMIND_DRAFT.set}
          </Button>
          <Button size="sm" onClick={(event: MouseEvent<HTMLButtonElement>) => setMenu(menuPoint(event))} aria-haspopup="menu" aria-expanded={menu !== null} data-testid="bot-reminder-change">
            {at === null ? REMIND_DRAFT.pick : REMIND_DRAFT.change}
          </Button>
        </div>
      )}
      {state.kind === "failed" && (
        <p role="alert" className="text-meta text-danger" data-testid="bot-reminder-failed">
          {REMIND_DRAFT.failed(state.why)}
        </p>
      )}
      {menu && (
        <RowMenu heading={REMIND_DRAFT.change} ariaLabel={REMIND_DRAFT.change} at={menu} onClose={() => setMenu(null)} testId="bot-reminder-menu">
          {REMIND_CHOICES(opened).map((choice) => (
            <MenuItem
              key={choice.label}
              onClick={() => {
                setMenu(null);
                setAt(Date.now() + choice.minutes * 60_000);
              }}
              testId="bot-reminder-choice"
            >
              <MenuLine icon={IconClockFace} label={choice.label} />
            </MenuItem>
          ))}
        </RowMenu>
      )}
    </div>
  );
}
