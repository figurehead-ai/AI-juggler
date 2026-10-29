/**
 * AJ-E2503 — one notice in the centre: an unread dot, its mark, what
 * happened in plain words, its one primary action (and the design's
 * second one beside it), and ⋯ with Snooze 1 h, Snooze until tomorrow,
 * Mute this project and Mute this kind (the shared row menu: icons on
 * every line, ↑↓, Esc). What happened earlier is one quieter line with
 * its time and its link. Every action is the user's click.
 */
import { useState, type JSX, type MouseEvent } from "react";
import { Button } from "./primitives";
import { MenuItem, MenuSeparator, RowMenu, menuPoint } from "./RowMenu";
import { IconMute, IconSnooze } from "./icons-actions";
import { IconAsk, IconClock, IconDone, IconStopped, IconWarn } from "./icons-menu";
import { ChannelChip } from "./ChannelChip";
import { happenedAt, needLine, needMeta, primaryLabel, secondOf, toneOf } from "../lib/notice-words";
import type { NoticeActions } from "../lib/notice-actions";
import type { NoticeInfo } from "../lib/ipc/notices";

const TONES = {
  ask: { className: "bg-accent-soft text-accent-hi", Icon: IconAsk },
  warn: { className: "bg-warning-soft text-warning", Icon: IconWarn },
  ok: { className: "bg-success-soft text-success", Icon: IconDone },
  err: { className: "bg-danger-soft text-danger", Icon: IconStopped },
  info: { className: "bg-live-soft text-live", Icon: IconClock },
};

/** A notice's mark (its tone and icon); `small` in the rail. */
export function NoticeMark({ kind, small = false }: { kind: string; small?: boolean }): JSX.Element {
  const tone = TONES[toneOf(kind)];
  return (
    <span aria-hidden="true" className={"flex shrink-0 items-center justify-center rounded-lg " + (small ? "size-6 " : "size-8 ") + tone.className}>
      <tone.Icon width={small ? 14 : 16} height={small ? 14 : 16} />
    </span>
  );
}

function Line({ icon: Icon, label }: { icon: typeof IconSnooze; label: string }): JSX.Element {
  return (
    <span className="flex items-center gap-2">
      <Icon width={14} height={14} aria-hidden="true" className="shrink-0 text-muted" />
      <span data-label>{label}</span>
    </span>
  );
}

export function NoticeRow({ notice, actions, now }: { notice: NoticeInfo; actions: NoticeActions; now: number }): JSX.Element {
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);
  // F16: a hand-over with no open session opens the account, and says so.
  const primary = notice.action === "hand-over" && !actions.canHandOver(notice) ? "Open its account" : primaryLabel(notice);
  const second = secondOf(notice);
  const meta = needMeta(notice);
  const pick = (run: () => void) => () => {
    setMenuAt(null);
    run();
  };
  if (!notice.needs) {
    return (
      <li className="flex gap-2.5 border-t border-hairline/50 px-3.5 py-2.5" data-testid="notice-earlier" data-kind={notice.kind}>
        <NoticeMark kind={notice.kind} />
        <div className="min-w-0 flex-1">
          <p className="flex flex-wrap items-center gap-2 text-body text-ink">
            {notice.unread && <span className="sr-only">Unread: </span>}
            {notice.title}
            {notice.kind === "update" && notice.facts.channel && <ChannelChip channel={notice.facts.channel} />}
          </p>
          <p className="flex flex-wrap items-center gap-x-1.5 text-label text-muted">
            <span>{happenedAt(notice.created_ms, now)}</span>
            {notice.detail && <span>· {notice.detail}</span>}
            {primary && (
              <>
                <span aria-hidden="true">·</span>
                <button type="button" onClick={() => actions.primary(notice)} className="rounded font-medium text-accent-hi underline-offset-2 hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
                  {primary}
                  <span className="sr-only"> — {notice.title}</span>
                </button>
              </>
            )}
          </p>
        </div>
        {notice.unread && <span aria-hidden="true" className="mt-2 size-2 shrink-0 rounded-full bg-accent" />}
      </li>
    );
  }
  return (
    <li className={"flex gap-2.5 border-t border-hairline/50 px-3.5 py-2.5 " + (notice.snoozed ? "opacity-70" : "")} data-testid="notice-need" data-kind={notice.kind}>
      <span aria-hidden="true" className={"mt-3 size-2 shrink-0 rounded-full " + (notice.unread ? "bg-accent" : "bg-transparent")} />
      <NoticeMark kind={notice.kind} />
      <div className="min-w-0 flex-1">
        <p className="text-body font-semibold text-ink">
          {notice.unread && <span className="sr-only">Unread: </span>}
          {notice.title}
        </p>
        <p className="text-meta text-muted">{needLine(notice, now)}</p>
        {meta && <p className="truncate text-label text-muted">{meta}</p>}
        <div className="mt-2 flex flex-wrap items-center gap-1.5">
          {primary && (
            <Button size="sm" variant="primary" onClick={() => actions.primary(notice)} data-testid="notice-primary">
              {primary}
              <span className="sr-only"> — {notice.title}</span>
            </Button>
          )}
          {second && (
            <Button size="sm" variant={notice.kind === "quota" ? "secondary" : "ghost"} onClick={() => actions.second(notice, second.id)} data-testid="notice-second">
              {second.label}
              <span className="sr-only"> — {notice.title}</span>
            </Button>
          )}
          <Button size="sm" variant="ghost" aria-label={`More for ${notice.title}`} aria-haspopup="menu" aria-expanded={menuAt !== null} onClick={(event: MouseEvent<HTMLButtonElement>) => setMenuAt(menuPoint(event))} data-testid="notice-more">
            ⋯
          </Button>
        </div>
      </div>
      {menuAt && (
        <RowMenu heading={notice.title} ariaLabel={`More for ${notice.title}`} at={menuAt} onClose={() => setMenuAt(null)}>
          {notice.snoozed && (
            <MenuItem onClick={pick(() => actions.wake(notice))} testId="notice-wake">
              <Line icon={IconSnooze} label="Back into Needs you now" />
            </MenuItem>
          )}
          <MenuItem onClick={pick(() => actions.snooze(notice, "hour"))} testId="notice-snooze-hour">
            <Line icon={IconSnooze} label="Snooze 1 h" />
          </MenuItem>
          <MenuItem onClick={pick(() => actions.snooze(notice, "tomorrow"))} testId="notice-snooze-tomorrow">
            <Line icon={IconSnooze} label="Snooze until tomorrow" />
          </MenuItem>
          <MenuSeparator />
          <MenuItem onClick={pick(() => actions.mute(notice, "project"))} disabled={!notice.project_id} title={notice.project_id ? undefined : "It belongs to no project"} testId="notice-mute-project">
            <Line icon={IconMute} label="Mute this project" />
          </MenuItem>
          <MenuItem onClick={pick(() => actions.mute(notice, "kind"))} testId="notice-mute-kind">
            <Line icon={IconMute} label="Mute this kind" />
          </MenuItem>
        </RowMenu>
      )}
    </li>
  );
}
