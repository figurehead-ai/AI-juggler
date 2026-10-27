/**
 * AJ-E2104 — the rail by place: NEEDS YOU, OPEN NOW grouped by place (app
 * and profile; or by account, or by pool once you've made one), the
 * "Also:" line, then the views turned on. The lines come from `lib/rail-lines.ts`; this draws them, only the
 * ones in view past about 30 rows, keeps rows still while the pointer or
 * focus is in the list, moves through them with ↑ and ↓, and takes ⌘1–9
 * to bring a place's app forward (stable: places never re-sort).
 *
 * D-45: Open now keeps the order picked on its header, Your order by
 * default (nothing moves by itself; rows and places are dragged, or moved
 * with ⌥⌘↑ and ⌥⌘↓); `lib/rail-order-store.ts` keeps it.
 */
import { useEffect, useMemo, useRef, useState, type JSX, type ReactNode } from "react";
import { NeedRow, PlaceHeader, PlaceRow, edgeOf, type Arrangeable } from "./PlaceRows";
import { AlsoLine, Chevron, SectionHead, scrollerOf } from "./PlaceBoardParts";
import { RailOrderMenu } from "./RailOrderMenu";
import { useArrange } from "./rail-arrange";
import { useActionHost } from "./ItemMenu";
import { AccountAvatar } from "./Avatar";
import { showToast } from "./Toast";
import { useShownConversations } from "../lib/shown-apps";
import { conversationsBringForward } from "../lib/ipc/conversations";
import { itemsOf, openNow, OWN_PLACE } from "../lib/rail-items";
import { buildLines, foldSections, LINE_HEIGHT, offsetOf, shownOrder, VIRTUALISE_AT, windowOf, type Line } from "../lib/rail-lines";
import type { RailGroup } from "../lib/rail-places";
import { dayOf, lensOf, LONG_GROUP, rankOf } from "../lib/rail-order";
import { reconcileOrder, toggleFold, useRailOrder } from "../lib/rail-order-store";
import { parseQuery, RAIL_REVEAL_EVENT, showInRail, useRailView } from "../lib/rail-view";
import { assignProfiles } from "../lib/profile-marks";
import { isSnoozed, type SnoozeState } from "../lib/railview";
import { useSeen } from "../lib/seen";
import { accountName, useSubscriptions } from "../lib/subscriptions";
import { railPoolOf, usePools } from "../lib/pools";
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

export function PlaceBoard({ sessions, projects, snooze, now, selectedId, onSelect, query, scroll }: BoardProps): JSX.Element {
  // Settings › Desktop apps: an app turned off leaves its count.
  const { conversations: all, hidden } = useShownConversations();
  const seen = useSeen();
  const view = useRailView();
  const order = useRailOrder();
  const parsed = useMemo(() => parseQuery(query), [query]);
  const pins = useMemo(() => new Set(order.pins), [order.pins]);
  const items = useMemo(
    () => itemsOf({ sessions, conversations: all, projects, seen, snoozed: (row) => isSnoozed(row, snooze, now), pins }),
    [sessions, all, projects, seen, snooze, now, pins],
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
  // AJ-E1702: the Pool lens, once a pool of yours exists (places before).
  const pools = usePools();
  const poolOf = useMemo(() => railPoolOf(pools), [pools]);
  const lens = view.group === "pool" && !poolOf ? "place" : view.group;
  const orderLens = lensOf(lens);
  const orderInputs = useMemo(
    () => ({ mode: order.mode, rows: rankOf(order.rows), groups: order.groups[orderLens], folded: new Set(order.folded) }),
    [order.mode, order.rows, order.groups, order.folded, orderLens],
  );
  const built = buildLines({ items, view, query: parsed, now, toggled, frozen, accountOf, poolOf, order: orderInputs });
  const stable = order.mode !== "latest";
  // New rows and places join the end of the record; gone ones are pruned.
  const groupKeys = built.groups.map((group) => group.key).join("\n");
  useEffect(() => {
    reconcileOrder({ known: items.map((item) => item.key), open: items.filter(openNow), lens: orderLens, groups: groupKeys === "" ? [] : groupKeys.split("\n"), today: dayOf(now) });
  }, [items, orderLens, groupKeys, now]);
  // Needs you and Open now fold in place from their heads.
  const [closed, setClosed] = useState<ReadonlySet<"needs" | "open">>(new Set());
  const lines = foldSections(built.lines, closed);
  const foldSection = (id: "needs" | "open") =>
    setClosed((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  useEffect(() => {
    last.current = shownOrder(built.lines);
  });
  const root = useRef<HTMLDivElement | null>(null);
  const freeze = () => setFrozen((current) => current ?? new Map(last.current));
  const thaw = () => {
    const active = document.activeElement;
    if (!root.current?.contains(active) && !root.current?.matches(":hover")) setFrozen(null);
  };

  // A header's own click: its profile's page here (an account's in the
  // Account lens, a pool's in the Pool lens); AI Juggler's own place is Home.
  const host = useActionHost();
  const pageOf = (group: RailGroup): (() => void) | undefined => {
    if (lens === "pool") {
      const pool = group.poolId;
      return pool !== null && host.openPool ? () => host.openPool!(pool) : undefined;
    }
    if (lens !== "account" && group.key === OWN_PLACE) return host.openHome;
    const id = group.accountId ?? group.items[0]?.accountId ?? null;
    return id !== null && host.openAccountById ? () => host.openAccountById!(id) : undefined;
  };
  const openPlace = (group: RailGroup) => {
    // A pool's header (and its ⌘ digit) opens the pool's page.
    if (group.poolId !== null) {
      host.openPool?.(group.poolId);
      return;
    }
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
  // D-45: dragging and ⌥⌘↑ / ⌥⌘↓ in Your order.
  const showAll = (key: string) => setToggled((current) => new Set(current).add(`${key}#all`));
  const arrange = useArrange({
    root,
    enabled: order.mode === "yours",
    lens: orderLens,
    groups: built.groups,
    arranged: built.arranged,
    drawn: (key) => (toggled.has(`${key}#all`) ? Infinity : LONG_GROUP),
    showAll,
  });
  const drag = arrange.drag;
  /** Where a drag would land, as a line on the row or header it lands by. */
  const dropAt = (keys: string[], self: string): Map<string, "before" | "after"> => {
    const out = new Map<string, "before" | "after">();
    if (!drag || drag.to === drag.from) return out;
    const others = keys.filter((key) => key !== self);
    if (drag.to < others.length) out.set(others[drag.to]!, "before");
    else if (others.length > 0) out.set(others[others.length - 1]!, "after");
    return out;
  };
  const rowDrop = drag?.kind === "row" && drag.group !== null ? dropAt(lines.filter((line) => line.kind === "row" && line.group.key === drag.group).map((line) => (line as { item: { key: string } }).item.key), drag.key) : new Map();
  const headDrop = drag?.kind === "group" ? dropAt(built.groups.map((group) => group.key), drag.key) : new Map();
  const arrangeOf = (kind: "row" | "group", group: string, key: string): Arrangeable => ({
    group,
    enabled: order.mode === "yours",
    grip: (event) => (kind === "row" ? arrange.gripRow(event, group, key) : arrange.gripGroup(event, group)),
    drop: (kind === "row" ? rowDrop : headDrop).get(key) ?? null,
    dragging: drag?.kind === kind && drag.key === key,
  });

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

  // "Also: n inactive" (or Home's link) turned a view on: the list
  // scrolls to it and its head takes focus, so the click shows something.
  const [reveal, setReveal] = useState<string | null>(null);
  useEffect(() => {
    const on = (event: Event) => setReveal((event as CustomEvent<{ key?: string }>).detail?.key ?? null);
    window.addEventListener(RAIL_REVEAL_EVENT, on);
    return () => window.removeEventListener(RAIL_REVEAL_EVENT, on);
  }, []);
  useEffect(() => {
    if (reveal === null) return;
    // The view's lines can come a render later; a view with nothing in it
    // stops waiting after a moment.
    const at = lines.findIndex((line) => line.kind === "label" && line.id === reveal);
    if (at < 0) {
      const stop = window.setTimeout(() => setReveal(null), 1000);
      return () => window.clearTimeout(stop);
    }
    setReveal(null);
    const box = root.current ? scrollerOf(root.current) : null;
    // Its head to the top of the list (past the drawn lines of a long
    // list, first to where it starts), and focus on it.
    const head = () => root.current?.querySelector<HTMLElement>(`[data-testid="rail-also-view-${reveal}"]`) ?? null;
    if (!head() && box) box.scrollTop = Math.max(0, (root.current?.offsetTop ?? 0) + offsetOf(lines, at) - 8);
    const shown = head();
    if (shown && box) box.scrollTop += shown.getBoundingClientRect().top - box.getBoundingClientRect().top - 8;
    shown?.querySelector("button")?.focus({ preventScroll: true });
  }, [reveal, lines]);

  // ↑ and ↓ move through rows and place headers; past the drawn ones, the
  // list scrolls a row and moves on.
  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (arrange.onKey(event)) return;
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    if (event.altKey || event.metaKey || event.ctrlKey) return;
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
        return <SectionHead id={line.id} count={line.count} open={!closed.has(line.id)} onToggle={() => foldSection(line.id)} extra={line.id === "open" ? <RailOrderMenu /> : undefined} />;
      case "need":
        return <NeedRow item={line.item} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} />;
      case "header": {
        const group = line.group;
        const own = group.key === OWN_PLACE;
        const opens =
          lens === "pool"
            ? group.poolId !== null && host.openPool !== undefined
            : lens !== "account" && (own ? group.items.some((item) => item.session) : group.place?.appPid !== null && group.place?.appPid !== undefined);
        return (
          <PlaceHeader
            place={lens === "account" || lens === "pool" ? null : group.place}
            label={group.label}
            count={group.items.length}
            needs={line.needs}
            folded={line.folded}
            // Your order and Name remember a fold; Latest activity's is this visit's.
            onToggle={() => (stable ? toggleFold(orderLens, group.key) : toggle(group.key))}
            arrange={arrangeOf("group", group.key, group.key)}
            onOpen={opens ? () => openPlace(group) : undefined}
            openWords={lens === "pool" ? "Open its page" : undefined}
            onPage={pageOf(group)}
            shortcut={line.digit ?? undefined}
          >
            {lens === "account" ? <AccountAvatar accountId={group.accountId} label={group.label} size={16} /> : undefined}
          </PlaceHeader>
        );
      }
      case "row":
        return (
          <PlaceRow
            item={line.item}
            edge={edgeOf(line.group.place)}
            selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)}
            onSelect={onSelect}
            now={now}
            subagents={view.show.subagents}
            tools
            arrange={arrangeOf("row", line.group.key, line.item.key)}
          />
        );
      case "plain":
        return <PlaceRow item={line.item} edge={edgeOf(line.item.place)} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} subagents={view.show.subagents} />;
      case "more":
        return (
          <button type="button" onClick={() => toggle(`${line.group.key}#all`)} data-testid="rail-more-quiet" className="ml-3 flex h-[24px] items-center rounded-md px-2.5 text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
            +{line.quiet} {line.words}
          </button>
        );
      case "also":
        return <AlsoLine also={built.also} hidden={hidden} />;
      case "label":
        return (
          <h3 className="flex h-[26px] items-end px-0.5 text-meta font-semibold text-muted" data-testid={`rail-also-view-${line.id}`}>
            {/* Folds it away again (the Also line's part turns it on). */}
            <button type="button" aria-expanded="true" data-rail-nav onClick={() => showInRail(line.id, false)} className="flex min-h-6 w-full items-center gap-1 rounded-md px-1.5 outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60">
              {line.id === "ended" ? "Ended today" : "Inactive"} <span className="tabular-nums">{line.count}</span>
              <Chevron open />
            </button>
          </h3>
        );
    }
  };

  const offset = root.current?.offsetTop ?? 0;
  const win = built.rows > VIRTUALISE_AT ? windowOf(lines, Math.max(0, scroll.top - offset), scroll.height) : null;
  const shown = win ? lines.slice(win.start, win.end) : lines;
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
      {/* D-45: each move said, politely ("Moved to position 2 of 5"). */}
      <p role="status" aria-live="polite" className="sr-only" data-testid="rail-order-said">
        {arrange.said}
      </p>
      {items.length === 0 && (
        <div className="flex flex-col gap-1.5 px-3 py-6" data-testid="rail-empty">
          <p className="text-label font-medium text-ink">Nothing open yet</p>
          <p className="text-label leading-relaxed text-muted">Start one with New session, or open one in your apps: it shows here.</p>
        </div>
      )}
    </div>
  );
}
