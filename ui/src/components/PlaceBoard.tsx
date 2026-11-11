/**
 * AJ-E2104 — the rail by place: NEEDS YOU, OPEN NOW grouped by place (app and profile; or by account,
 * or by pool once you've made one), the "Also:" line, then the views turned on. The lines come from
 * `lib/rail-lines.ts`; this draws them, only the ones in view past about 30 rows, keeps rows still
 * while the pointer or focus is in the list (and in place: rail-hold), moves through them with ↑
 * and ↓, and takes ⌘1–9 to bring a place's app forward (stable: places never re-sort).
 *
 * D-47: Open now keeps the order picked on its header, Your order by
 * default (nothing moves by itself; rows and places are dragged, or moved
 * with ⌥⌘↑ and ⌥⌘↓); `lib/rail-order-store.ts` keeps it.
 */
import { useEffect, useLayoutEffect, useMemo, useReducer, useRef, useState, type JSX, type ReactNode } from "react";
import { NeedRow, PlaceHeader, edgeOf, type Arrangeable } from "./PlaceRows";
import { SessionRow } from "./row/SessionRow";
import { AlsoLine, Chevron, RailLink, RailSearchFoundNothing, SectionHead, scrollerOf, stepPastEdge } from "./PlaceBoardParts";
import { RailIdleApp } from "./RailIdleApp";
import { RailOfferRow, usePlaceOffers } from "./PlaceOffers";
import { RailOrderMenu } from "./RailOrderMenu";
import { useArrange } from "./rail-arrange";
import { LINE_KEY, useHoldStill } from "./rail-hold";
import { useActionHost } from "./ItemMenu";
import { IconAccount } from "./usage/controls";
import { useMarks } from "../lib/marks";
import { showToast } from "./Toast";
import { showGoTo } from "../lib/go-to";
import { useShownConversations } from "../lib/shown-apps";
import { conversationsBringForward } from "../lib/ipc/conversations";
import { useKnownPlaces } from "../lib/places";
import { itemsOf, openNow } from "../lib/rail-items";
import { useClosedSections } from "./rail-groups";
import { buildLines, foldSections, lineHeights, lineKey, offsetOf, shownOrder, VIRTUALISE_AT, windowOf, type Line } from "../lib/rail-lines";
import { headerActions, keepAliveOf } from "./place-actions";
import { recentWords } from "../lib/rail-apps";
import { groupApp, groupClosed, openGroupApp } from "./rail-group-app";
import { appIdOfRailKey, openApp } from "../lib/apps";
import { openAccount } from "../lib/accounts-page";
import { accountOfProfile, cachedSubscriptions } from "../lib/subscriptions";
import { renamedPlaces, type RailGroup } from "../lib/rail-places";
import { dayOf, lensOf, LONG_GROUP, rankOf } from "../lib/rail-order";
import { useOffRail } from "../lib/rail-shown";
import { reconcileOrder, toggleFold, useRailOrder } from "../lib/rail-order-store";
import { parseQuery, RAIL_REVEAL_EVENT, showInRail, useRailView } from "../lib/rail-view";
import { isSnoozed, type SnoozeState } from "../lib/railview";
import { useBackendNeeds } from "../lib/needs-you";
import { useNoticeActions } from "../lib/notice-actions";
import { RailNoticeRow } from "./RailNoticeRow";
import { useSeen } from "../lib/seen";
import { onLine, roveKey } from "../lib/roving";
import { useRailAccountOf } from "../lib/rail-accounts";
import { railPoolOf, usePools, warmPool } from "../lib/pools";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

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
  /** Settings › Appearance › Density (AJ-E403): Compact rows are one line. */
  density?: "comfortable" | "compact";
  /** 0.4.1: the box above the list where the "new above" pill takes its own space. */
  cueSlot?: HTMLElement | null;
}

export function PlaceBoard({ sessions, projects, snooze, now, selectedId, onSelect, query, scroll, density = "comfortable", cueSlot = null }: BoardProps): JSX.Element {
  const compact = density === "compact";
  const heights = lineHeights(compact);
  // Settings › Desktop apps: an app turned off leaves its count.
  const { conversations: all, hidden } = useShownConversations();
  const seen = useSeen();
  const view = useRailView();
  const order = useRailOrder();
  const parsed = useMemo(() => parseQuery(query), [query]);
  // D-49: Needs you is the backend's: its sessions, and the rest beside.
  const needsNow = useBackendNeeds();
  const actions = useNoticeActions(sessions);
  const pins = useMemo(() => new Set(order.pins), [order.pins]);
  const items = useMemo(
    () => itemsOf({ sessions, conversations: all, projects, seen, snoozed: (row) => isSnoozed(row, snooze, now), pins, needOf: needsNow.needOf }),
    [sessions, all, projects, seen, snooze, now, pins, needsNow.needOf],
  );
  const shownKeys = new Set(items.map((item) => item.key));
  const others = needsNow.rows.filter((row) => !needsNow.needOf.has(row.subject) || !shownKeys.has(row.subject));
  useMarks(); // the rows' edges follow what the lists say of each App (AJ-E3206)
  const [toggled, setToggled] = useState<ReadonlySet<string>>(new Set());
  const [, redraw] = useReducer((n: number) => n + 1, 0);
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
  // The Account lens groups by subscription (AJ-E2109); `account:` means one (D-62).
  const accountOf = useRailAccountOf(view.group === "account" || view.only.account !== undefined || parsed.only.account !== undefined);
  // AJ-E1702: the Pool lens, once a pool of yours exists (places before).
  const pools = usePools();
  const poolOf = useMemo(() => railPoolOf(pools), [pools]);
  const lens = view.group === "pool" && !poolOf ? "place" : view.group;
  const orderLens = lensOf(lens);
  const orderInputs = useMemo(
    () => ({ mode: order.mode, rows: rankOf(order.rows), groups: order.groups[orderLens], folded: new Set(order.folded) }),
    [order.mode, order.rows, order.groups, order.folded, orderLens],
  );
  // The apps remembered (0.3.3 Keep alive, D-64 inactive apps); AJ-E3204: the ones kept off.
  const places = useKnownPlaces();
  const { offRail, openOffRail } = useOffRail(places);
  const offers = usePlaceOffers(); // D-100: Running, not added; an Add that failed says why
  useEffect(() => void (offers.note && showToast({ id: "place-offer", message: offers.note })), [offers.note]);
  const built = buildLines({ items, view, query: parsed, now, toggled, frozen, accountOf, poolOf, order: orderInputs, others, known: places ?? [], offRail, offers: offers.offers ?? [] });
  const stable = order.mode !== "latest";
  // New rows and places join the end of the record; gone ones are pruned.
  const groupKeys = built.groups.map((group) => group.key).join("\n");
  // D-62: a place kept by 0.3's key (app and profile) is the App it became.
  // (As words, so the record is brought up to date only when they change.)
  const renamed = lens === "place" ? [...renamedPlaces(built.groups)].map(([old, now]) => `${old}\t${now}`).join("\n") : "";
  useEffect(() => {
    const pairs = renamed === "" ? [] : renamed.split("\n").map((line) => line.split("\t") as [string, string]);
    reconcileOrder({ known: items.map((item) => item.key), open: items.filter(openNow), lens: orderLens, groups: groupKeys === "" ? [] : groupKeys.split("\n"), today: dayOf(now), renamed: new Map(pairs) });
  }, [items, orderLens, groupKeys, now, renamed]);
  // Needs you, Open now and Inactive apps fold in place from their heads;
  // Inactive apps starts folded, and each one stays as it was left.
  const [closed, foldSection] = useClosedSections();
  const lines = foldSections(built.lines, closed);
  useEffect(() => {
    last.current = shownOrder(built.lines);
  });
  const root = useRef<HTMLDivElement | null>(null);
  const freeze = () => setFrozen((current) => current ?? new Map(last.current));
  const thaw = () => {
    const active = document.activeElement;
    if (!root.current?.contains(active) && !root.current?.matches(":hover")) setFrozen(null);
  };
  const hold = useHoldStill(root, { enter: freeze, leave: thaw, slot: cueSlot });

  // A header's own click: its App's page here (its Account's in the
  // Account lens, a pool's in the Pool lens); AI Juggler's own place is Home.
  const host = useActionHost();
  const pageOf = (group: RailGroup): (() => void) | undefined => {
    if (lens === "pool") {
      const pool = group.poolId;
      return pool !== null && host.openPool ? () => host.openPool!(pool) : undefined;
    }
    // D-62: grouped by App, the App's own page.
    if (lens === "place" && group.place) {
      const app = appIdOfRailKey(group.place.key, group.items[0]?.accountId ?? null, places);
      if (app !== null) return () => openApp(app);
    }
    const id = group.accountId ?? group.items[0]?.accountId ?? null;
    // D-62: grouped by Account, the Account's own page.
    const account = lens === "account" && id !== null ? accountOfProfile(cachedSubscriptions() ?? [], id)?.account_id : null;
    if (account) return () => openAccount(account);
    return id !== null && host.openAccountById ? () => host.openAccountById!(id) : undefined;
  };
  const knownOf = (group: RailGroup) => groupApp(lens, group, places); // 0.4.1: by key, running or not
  const openPlace = (group: RailGroup) => {
    // A pool's header (and its ⌘ digit) opens the pool's page.
    if (group.poolId !== null) {
      host.openPool?.(group.poolId);
      return;
    }
    const known = knownOf(group);
    // Closed: started (0.4.1); running: brought forward as before.
    if (groupClosed(known, group)) return openGroupApp(known);
    // A CLI App's sessions: the first leads (selected here; else its window comes forward).
    const first = group.items[0];
    if (first?.session && first.place.kind === "own") {
      onSelect(first.session.id);
      return;
    }
    const pid = group.items.find((item) => item.place.appPid !== null)?.place.appPid;
    if (pid === null || pid === undefined) return void (known && openGroupApp(known));
    showToast({ id: "go-to", message: "Bringing it forward…" });
    conversationsBringForward(pid).then(showGoTo, () => showToast({ id: "go-to", message: "Couldn't bring it forward" }));
  };
  // D-47: dragging and ⌥⌘↑ / ⌥⌘↓ in Your order.
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
  useLayoutEffect(() => {
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
    // The lines can come a render later; an empty view stops waiting after a
    // moment. D-64: by App, Home's "inactive" goes to Inactive apps.
    const idleHead = reveal === "inactive" && lens === "place";
    const at = lines.findIndex((line) => (idleHead ? line.kind === "section" && line.id === "idle" : line.kind === "label" && line.id === reveal));
    if (at < 0) {
      const stop = window.setTimeout(() => setReveal(null), 1000);
      return () => window.clearTimeout(stop);
    }
    setReveal(null);
    const box = root.current ? scrollerOf(root.current) : null;
    // Its head to the top of the list (past the drawn lines of a long
    // list, first to where it starts), and focus on it.
    const head = () => root.current?.querySelector<HTMLElement>(idleHead ? '[data-testid="rail-inactive-apps"]' : `[data-testid="rail-also-view-${reveal}"]`) ?? null;
    if (!head() && box) box.scrollTop = Math.max(0, (root.current?.offsetTop ?? 0) + offsetOf(lines, at, heights) - 8);
    const shown = head();
    if (shown && box) box.scrollTop += shown.getBoundingClientRect().top - box.getBoundingClientRect().top - 8;
    (shown?.matches("button") ? shown : shown?.querySelector("button"))?.focus({ preventScroll: true });
    return undefined;
  }, [reveal, lines]);

  // ↑ and ↓ move through the lines (lib/roving: rows, place headers, the
  // Also line); past the drawn ones, the list scrolls a row and moves on.
  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (arrange.onKey(event)) return;
    // The rail's list moved it already (lib/roving, on the list around).
    if (event.defaultPrevented) return;
    const box = root.current;
    if (!box || (event.key !== "ArrowDown" && event.key !== "ArrowUp")) return;
    if (roveKey(box, event)) {
      event.preventDefault();
      return;
    }
    // At the first or last line drawn of a long list: it scrolls a row,
    // and on (a short list's end is the rail's to pass, lib/roving).
    if (!win || event.altKey || event.metaKey || event.ctrlKey || event.shiftKey || !scroll.box || !onLine(box, event.target)) return;
    event.preventDefault();
    stepPastEdge(box, scroll.box, event.key === "ArrowDown" ? 1 : -1, heights.row, redraw);
  };

  const draw = (line: Line): ReactNode => {
    switch (line.kind) {
      case "section":
        return <SectionHead id={line.id} count={line.count} open={!closed.has(line.id)} onToggle={() => foldSection(line.id)} extra={line.id === "open" ? <RailOrderMenu /> : undefined} />;
      case "need":
        return <NeedRow item={line.item} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} compact={compact} />;
      case "notice":
        return <RailNoticeRow notice={line.notice} now={now} onOpen={actions.primary} canHandOver={actions.canHandOver(line.notice)} />;
      case "header": {
        const group = line.group;
        const known = knownOf(group);
        const opens =
          lens === "pool"
            ? group.poolId !== null && host.openPool !== undefined
            : lens !== "account" && (known !== null || group.items.some((item) => (item.session !== null && item.place.kind === "own") || item.place.appPid !== null));
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
            openWords={lens === "pool" ? "Open its page" : groupClosed(known, group) ? "Start" : undefined}
            onPage={pageOf(group)} onWarm={lens === "pool" && group.poolId !== null ? () => void warmPool(group.poolId!) : undefined}
            shortcut={line.digit ?? undefined}
            onNew={lens === "place" && group.key.startsWith("cli|") ? () => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: group.items[0]?.accountId ?? null } })) : undefined}
            {...headerActions(known, group)} roomOf={lens === "pool" ? null : group.items[0]?.accountId}
          >
            {/* Grouped by Account: its icon, not a colour (colour is the Apps' in the rows below). */}
            {lens === "account" ? <IconAccount width={14} height={14} aria-hidden="true" className="shrink-0 text-muted" data-testid="rail-account-icon" /> : undefined}
          </PlaceHeader>
        );
      }
      case "row":
        return (
          <SessionRow
            item={line.item}
            density={compact ? "compact" : "cozy"}
            within={{ app: true }}
            edge={edgeOf(line.group.place)}
            selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)}
            onSelect={onSelect}
            now={now}
            subagents={view.show.subagents}
            tools
            arrange={arrangeOf("row", line.group.key, line.item.key)}
            depth={line.depth} parent={line.parent}
          />
        );
      case "plain":
        return <SessionRow item={line.item} density={compact ? "compact" : "cozy"} within={{ app: true }} edge={edgeOf(line.item.place)} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} subagents={view.show.subagents} />;
      case "more":
        return <RailLink onClick={() => toggle(`${line.group.key}#all`)} testId="rail-more-quiet">{`+${line.quiet} ${line.words}`}</RailLink>;
      case "recent":
        return <RailLink onClick={() => toggle(`${line.app}#recent`)} testId="rail-recent" expanded={line.open}>{recentWords(line.count)}</RailLink>;
      case "past":
        return <SessionRow item={line.item} density={compact ? "compact" : "cozy"} within={{ app: true }} edge={edgeOf(line.item.place)} selected={selectedId === (line.item.session?.id ?? line.item.conversation?.id)} onSelect={onSelect} now={now} subagents={view.show.subagents} depth={line.depth} parent={line.parent} />;
      case "idle": {
        const known = line.app.placeId !== null ? (places?.find((place) => place.id === line.app.placeId) ?? null) : null;
        return <RailIdleApp app={line.app} open={line.open} onToggle={() => toggle(`${line.app.key}#recent`)} now={now} keepAlive={keepAliveOf(known)} />;
      }
      case "idle-more":
        return <RailLink onClick={() => toggle("idle#all")} testId="rail-idle-more">{`Show ${line.hidden} more`}</RailLink>;
      case "offer":
        return <RailOfferRow offer={line.offer} onAnswer={(add) => offers.answer(line.offer, add)} />;
      case "also":
        return <AlsoLine also={built.also} hidden={hidden} offRail={built.offRail} openApp={openOffRail} />;
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
  const top = scroll.box?.scrollTop ?? scroll.top; // the box's own, read now (a redraw comes before its event)
  const win = built.rows > VIRTUALISE_AT ? windowOf(lines, Math.max(0, top - offset), scroll.height, 240, heights) : null;
  const shown = win ? lines.slice(win.start, win.end) : lines;
  return (
    <div
      ref={root}
      data-testid="place-board"
      data-density={density}
      data-frozen={frozen ? "true" : undefined}
      {...hold.props}
      onKeyDown={onKeyDown}
      className="flex flex-col"
    >
      {hold.before}<RailSearchFoundNothing query={query} anything={items.length > 0} lines={built.lines} />
      {win && <div style={{ height: win.before }} aria-hidden="true" />}
      {shown.map((line, index) => (
        <div key={lineKey(line, index)} data-rail-line {...{ [LINE_KEY]: lineKey(line, index) }} style={{ minHeight: heights[line.kind] }} className="flex flex-col justify-center">
          {draw(line)}
        </div>
      ))}
      {win && <div style={{ height: win.after }} aria-hidden="true" />}
      {hold.after}
      {/* D-47: each move said, politely ("Moved to position 2 of 5"). */}
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
