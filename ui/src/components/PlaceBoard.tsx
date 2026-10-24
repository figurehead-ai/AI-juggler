/**
 * AJ-E2104 — the rail by place: NEEDS YOU, OPEN NOW grouped by place (app
 * and profile; or by account), the "Also:" line, then the views turned
 * on. The lines come from `lib/rail-lines.ts`; this draws them, only the
 * ones in view past about 30 rows, keeps rows still while the pointer or
 * focus is in the list, moves through them with ↑ and ↓, and takes ⌘1–9
 * to bring a place's app forward (stable: places never re-sort).
 */
import { Fragment, useEffect, useMemo, useRef, useState, type JSX, type ReactNode } from "react";
import { NeedRow, PlaceHeader, PlaceRow, edgeOf } from "./PlaceRows";
import { AccountAvatar } from "./Avatar";
import { IconAlert } from "./icons";
import { IconWindow } from "./icons-places";
import { showToast } from "./Toast";
import { useShownConversations, type HiddenApp } from "../lib/shown-apps";
import { conversationsBringForward } from "../lib/ipc/conversations";
import { itemsOf, OWN_PLACE } from "../lib/rail-items";
import { buildLines, LINE_HEIGHT, shownOrder, VIRTUALISE_AT, windowOf, type AlsoCounts, type Line } from "../lib/rail-lines";
import type { RailGroup } from "../lib/rail-places";
import { parseQuery, showInRail, useRailView, type ShowKey } from "../lib/rail-view";
import { assignProfiles } from "../lib/profile-marks";
import { isSnoozed, type SnoozeState } from "../lib/railview";
import { useSeen } from "../lib/seen";
import { accountName, useSubscriptions } from "../lib/subscriptions";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

interface BoardProps {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  snooze: SnoozeState;
  now: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
  query: string;
  /** The scroll box's top and height, and the box (to bring a row into view). */
  scroll: { top: number; height: number; box: HTMLElement | null };
}

const SECTION: Record<"needs" | "open", { label: string; tone: string; Icon: typeof IconAlert }> = {
  needs: { label: "Needs you", tone: "text-warning", Icon: IconAlert },
  open: { label: "Open now", tone: "text-live", Icon: IconWindow },
};

function SectionHead({ id, count }: { id: "needs" | "open"; count: number }): JSX.Element {
  const { label, tone, Icon } = SECTION[id];
  return (
    <h3 className={"flex h-[28px] items-center gap-1.5 px-2 pt-1 text-[10px] font-semibold uppercase tracking-[0.08em] " + tone} data-testid={id === "needs" ? "rail-needs-you" : "rail-open-now"}>
      <Icon width={12} height={12} aria-hidden="true" />
      {label}
      <span className="tabular-nums">{count}</span>
    </h3>
  );
}

const ALSO: { key: keyof AlsoCounts; show: ShowKey; words: string }[] = [
  { key: "ended", show: "ended", words: "ended today" },
  { key: "inactive", show: "inactive", words: "inactive" },
  { key: "background", show: "background", words: "background" },
];

/** "Also: n ended today · n inactive · n background · History ›": each
 * part turns its view on (and off). */
function AlsoLine({ also, hidden }: { also: AlsoCounts; hidden: HiddenApp[] }): JSX.Element {
  const view = useRailView();
  // 24 px targets (WCAG 2.5.8), the line wrapping when it must.
  const link = "inline-flex min-h-6 items-center rounded px-0.5 outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";
  return (
    <p className="flex min-h-[30px] flex-wrap items-center gap-x-1 px-2 text-[11px] text-muted" data-testid="rail-also">
      <span>Also:</span>
      {ALSO.filter((part) => also[part.key] > 0).map((part) => (
        <Fragment key={part.key}>
          <button type="button" aria-pressed={view.show[part.show]} data-testid={`rail-also-${part.key}`} onClick={() => showInRail(part.show, !view.show[part.show])} className={link + (view.show[part.show] ? " text-accent-hi" : "")}>
            {also[part.key]} {part.words}
          </button>
          <span aria-hidden="true">·</span>
        </Fragment>
      ))}
      {hidden.map((part) => (
        <Fragment key={part.app}>
          <button type="button" data-testid="rail-also-hidden" className={link} title="Settings › Desktop apps" onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "settings" } }))}>
            {part.count} in {part.app} hidden · Settings
          </button>
          <span aria-hidden="true">·</span>
        </Fragment>
      ))}
      <button type="button" data-testid="rail-also-history" className={link} onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history" } }))}>
        History ›
      </button>
    </p>
  );
}

export function PlaceBoard({ sessions, projects, snooze, now, selectedId, onSelect, query, scroll }: BoardProps): JSX.Element {
  // Settings › Desktop apps: an app turned off leaves its count.
  const { conversations: all, hidden } = useShownConversations();
  const seen = useSeen();
  const view = useRailView();
  const parsed = useMemo(() => parseQuery(query), [query]);
  const items = useMemo(
    () => itemsOf({ sessions, conversations: all, projects, seen, snoozed: (row) => isSnoozed(row, snooze, now) }),
    [sessions, all, projects, seen, snooze, now],
  );
  assignProfiles(items.map((item) => item.place.profile ?? ""));
  const [toggled, setToggled] = useState<ReadonlySet<string>>(new Set());
  const toggle = (key: string) =>
    setToggled((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  // Rows hold still while the pointer or focus is in the list.
  const last = useRef<Map<string, string[]>>(new Map());
  const [frozen, setFrozen] = useState<Map<string, string[]> | null>(null);
  // The Account lens groups by subscription (AJ-E2109), read only for it.
  const subscriptions = useSubscriptions(0, view.group === "account");
  const accountOf = useMemo(() => {
    const byProfile = new Map((subscriptions ?? []).flatMap((row) => row.profiles.map((profile) => [profile.id, { key: row.key, label: accountName(row) }] as const)));
    return (profileId: string | null) => (profileId === null ? null : (byProfile.get(profileId) ?? null));
  }, [subscriptions]);
  const built = buildLines({ items, view, query: parsed, now, toggled, frozen, accountOf });
  useEffect(() => {
    last.current = shownOrder(built.lines);
  });
  const root = useRef<HTMLDivElement | null>(null);
  const freeze = () => setFrozen((current) => current ?? new Map(last.current));
  const thaw = () => {
    const active = document.activeElement;
    if (!root.current?.contains(active) && !root.current?.matches(":hover")) setFrozen(null);
  };

  const openPlace = (group: RailGroup) => {
    if (group.key === OWN_PLACE) {
      const first = group.items.find((item) => item.session);
      if (first?.session) onSelect(first.session.id);
      return;
    }
    const pid = group.place?.appPid;
    if (pid === null || pid === undefined) return;
    showToast({ id: "go-to", message: "Bringing it forward…" });
    conversationsBringForward(pid).then(
      (outcome) => showToast({ id: "go-to", message: outcome.outcome }),
      () => showToast({ id: "go-to", message: "Couldn't bring it forward" }),
    );
  };
  // ⌘1–9 (App's keys): the place in that position, in the stable order.
  // The listener reads the latest lines (a key can come before effects run).
  const latest = useRef({ groups: built.groups, openPlace });
  latest.current = { groups: built.groups, openPlace };
  useEffect(() => {
    const go = (event: Event) => {
      const index = (event as CustomEvent<{ index: number }>).detail?.index ?? -1;
      const group = latest.current.groups[index];
      if (group) latest.current.openPlace(group);
    };
    window.addEventListener("aij:go-place", go);
    return () => window.removeEventListener("aij:go-place", go);
  }, []);

  // ↑ and ↓ move through rows and place headers; past the drawn ones, the
  // list scrolls a row and moves on.
  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const targets = Array.from(root.current?.querySelectorAll<HTMLElement>("[data-rail-item], [data-rail-nav]") ?? []);
    const at = targets.indexOf(document.activeElement as HTMLElement);
    if (at < 0) return;
    event.preventDefault();
    const step = event.key === "ArrowDown" ? 1 : -1;
    const next = targets[at + step];
    if (next) next.focus();
    else if (scroll.box) {
      scroll.box.scrollTop += step * LINE_HEIGHT.row;
      requestAnimationFrame(() => {
        const again = Array.from(root.current?.querySelectorAll<HTMLElement>("[data-rail-item], [data-rail-nav]") ?? []);
        again[step > 0 ? again.length - 1 : 0]?.focus();
      });
    }
  };

  const draw = (line: Line): ReactNode => {
    switch (line.kind) {
      case "section":
        return <SectionHead id={line.id} count={line.count} />;
      case "need":
        return <NeedRow item={line.item} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} />;
      case "header": {
        const group = line.group;
        const own = group.key === OWN_PLACE;
        const opens = view.group !== "account" && (own ? group.items.some((item) => item.session) : group.place?.appPid !== null && group.place?.appPid !== undefined);
        return (
          <PlaceHeader
            place={view.group === "account" ? null : group.place}
            label={group.label}
            count={group.items.length}
            needs={line.needs}
            folded={line.folded}
            onToggle={() => toggle(group.key)}
            onOpen={opens ? () => openPlace(group) : undefined}
            shortcut={line.digit ?? undefined}
          >
            {view.group === "account" ? <AccountAvatar accountId={group.accountId} label={group.label} size={16} /> : undefined}
          </PlaceHeader>
        );
      }
      case "row":
        return <PlaceRow item={line.item} edge={edgeOf(line.group.place)} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} subagents={view.show.subagents} />;
      case "plain":
        return <PlaceRow item={line.item} edge={edgeOf(line.item.place)} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} subagents={view.show.subagents} />;
      case "more":
        return (
          <button type="button" onClick={() => toggle(`${line.group.key}#all`)} data-testid="rail-more-quiet" className="ml-3 flex h-[24px] items-center rounded-md px-2.5 text-[11px] text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
            +{line.quiet} quiet
          </button>
        );
      case "also":
        return <AlsoLine also={built.also} hidden={hidden} />;
      case "label":
        return (
          <p className="flex h-[26px] items-end px-2 pb-0.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-muted" data-testid={`rail-also-view-${line.id}`}>
            {line.id === "ended" ? "Ended today" : "Inactive"} <span className="ml-1 tabular-nums">{line.count}</span>
          </p>
        );
    }
  };

  const offset = root.current?.offsetTop ?? 0;
  const win = built.rows > VIRTUALISE_AT ? windowOf(built.lines, Math.max(0, scroll.top - offset), scroll.height) : null;
  const shown = win ? built.lines.slice(win.start, win.end) : built.lines;
  const keyOf = (line: Line, index: number): string =>
    line.kind === "need" || line.kind === "plain" ? `${line.kind}:${line.item.key}` : line.kind === "row" ? `row:${line.item.key}` : line.kind === "header" || line.kind === "more" ? `${line.kind}:${line.group.key}` : `${line.kind}:${"id" in line ? line.id : index}`;
  return (
    <div
      ref={root}
      data-testid="place-board"
      data-frozen={frozen ? "true" : undefined}
      onPointerEnter={freeze}
      onPointerLeave={() => setTimeout(thaw)}
      onFocus={freeze}
      onBlur={() => setTimeout(thaw)}
      onKeyDown={onKeyDown}
      className="flex flex-col"
    >
      {win && <div style={{ height: win.before }} aria-hidden="true" />}
      {shown.map((line, index) => (
        <div key={keyOf(line, index)} style={{ minHeight: LINE_HEIGHT[line.kind] }} className="flex flex-col justify-center">
          {draw(line)}
        </div>
      ))}
      {win && <div style={{ height: win.after }} aria-hidden="true" />}
      {items.length === 0 && (
        <div className="flex flex-col gap-1.5 px-3 py-6" data-testid="rail-empty">
          <p className="text-[12px] font-medium text-ink">Nothing open yet</p>
          <p className="text-[11px] leading-relaxed text-muted">Start one with New session, or open one in your apps: it shows here.</p>
        </div>
      )}
    </div>
  );
}
