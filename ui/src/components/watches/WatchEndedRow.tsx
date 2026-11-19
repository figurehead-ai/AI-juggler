/**
 * AJ-E3816 — an ended watch: its echo, why it ended in the code's own
 * sentence with one fact beside it, when, how often it told you, and its
 * way forward (ADR-0036 §2.3, never a silent failure): watch the App
 * instead once its session closed, renew, renew for today, pick a new
 * target once what it watched was removed, or only remove when AI Juggler
 * can no longer watch for it (D-127).
 */
import type { JSX } from "react";
import { clock } from "../../lib/time-words";
import type { WatchInfo } from "../../types/generated/WatchInfo";
import { EchoHead, WatchMarks } from "./WatchParts";
import { endedWhen, optionOf, type TargetOption } from "./watches-view";
import { ENDED, KIND } from "./watches-words";
import { IconBin, IconCheckCircle, IconInfoCircle, IconStopSquare, IconSun, type WatchIcon } from "./watch-icons";

export type EndedWay = { kind: "app"; appId: string; label: string } | { kind: "renew" } | { kind: "today" } | { kind: "retarget" } | null;

export interface EndedActions {
  forward: (info: WatchInfo, way: Exclude<EndedWay, null>) => void;
  remove: (info: WatchInfo) => void;
}

const REASON_ICONS: Record<string, WatchIcon> = {
  targets_ended: IconStopSquare,
  first_fire: IconCheckCircle,
  time_reached: IconSun,
  target_gone: IconBin,
  no_longer_offered: IconInfoCircle,
};

/** Its way forward, by why it ended. */
export function wayOf(info: WatchInfo, appOf: (sessionId: string) => { id: string; name: string } | null): EndedWay {
  switch (info.ended) {
    case "targets_ended": {
      const session = info.watch.targets.find((t) => t.kind === "session");
      const app = session && session.kind === "session" ? appOf(session.id) : null;
      return app ? { kind: "app", appId: app.id, label: ENDED.watchInstead(app.name) } : { kind: "renew" };
    }
    case "first_fire":
      return { kind: "renew" };
    case "time_reached":
      return { kind: "today" };
    case "target_gone":
      return { kind: "retarget" };
    case "no_longer_offered":
      return null;
    default:
      return { kind: "renew" };
  }
}

/** The one fact beside why it ended. */
function factOf(info: WatchInfo, book: Map<string, TargetOption>): string | null {
  const name = info.echo.head.find((span) => span.name)?.text ?? null;
  const first = info.watch.targets[0];
  switch (info.ended) {
    case "targets_ended":
      return name && info.endedMs !== null ? ENDED.closedAt(name, clock(info.endedMs)) : null;
    case "time_reached": {
      // Its own words for when it was to end: midnight is "Until midnight".
      const life = info.watch.lifetime;
      const midnight = life.kind === "until" && new Date(life.atMs).getHours() === 0 && new Date(life.atMs).getMinutes() === 0;
      return ENDED.wasLifetime(midnight ? ENDED.untilMidnight : info.echo.lifetime);
    }
    case "target_gone": {
      if (!first || first.kind === "any") return null;
      const kind = KIND[first.kind].toLowerCase();
      return name ? ENDED.removedTarget(kind, name) : null;
    }
    case "no_longer_offered":
      return first ? ENDED.noLonger(name ?? optionOf(book, first).name) : null;
    default:
      return null;
  }
}

function wayLabel(way: Exclude<EndedWay, null>): string {
  if (way.kind === "app") return way.label;
  if (way.kind === "today") return ENDED.renewToday;
  if (way.kind === "retarget") return ENDED.pickNew;
  return ENDED.renew;
}

export function WatchEndedRow({ info, book, now, way, actions }: { info: WatchInfo; book: Map<string, TargetOption>; now: number; way: EndedWay; actions: EndedActions }): JSX.Element {
  const options = info.watch.targets.map((target) => optionOf(book, target));
  const Icon = REASON_ICONS[info.ended ?? ""] ?? IconInfoCircle;
  const fact = factOf(info, book);
  const gone = info.ended === "target_gone";
  return (
    <li className="grid grid-cols-[44px_minmax(0,1fr)_minmax(112px,132px)_auto] items-center gap-x-3.5 px-3 py-3 [&+&]:border-t [&+&]:border-divider" data-testid="watch-ended-row" data-watch-id={info.watch.id} data-reason={info.ended ?? ""}>
      <WatchMarks options={options} ghost={gone} />
      <div className="min-w-0">
        <p className="text-body text-muted" data-testid="watch-echo">
          <EchoHead spans={info.echo.head} tone="quiet" />
        </p>
        <p className="mt-1.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-meta text-ink-2" data-testid="watch-ended-note">
          <Icon width={14} height={14} aria-hidden="true" className="shrink-0 text-faint" />
          <span>{info.endedWords}</span>
          {fact && <span className="text-muted">· {fact}</span>}
        </p>
      </div>
      <span className="flex flex-col gap-px text-meta">
        <span className="font-medium text-ink">{endedWhen(info, now)}</span>
        <span className="text-muted">{ENDED.told(info.told)}</span>
      </span>
      <div className="flex items-center justify-end gap-1.5">
        {way && (
          <button type="button" onClick={() => actions.forward(info, way)} className="inline-flex h-7 max-w-[220px] items-center rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-ended-forward" data-way={way.kind}>
            <span className="truncate">{wayLabel(way)}</span>
          </button>
        )}
        <button type="button" onClick={() => actions.remove(info)} aria-label={ENDED.remove(info.words)} title={ENDED.remove(info.words)} className="inline-flex size-7 items-center justify-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-danger focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-ended-remove">
          <IconBin width={15} height={15} aria-hidden="true" />
        </button>
      </div>
    </li>
  );
}
