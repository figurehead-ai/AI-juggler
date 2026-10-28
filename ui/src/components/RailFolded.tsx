/**
 * The rail folded to icons (⌘B, and by itself below the narrow width): the
 * glance (§1.4 L1). The 0.2 walk (F2) found it saying "All clear" while
 * the bell said four things needed you, with no session in it: it only
 * knew AI Juggler's own sessions, and counted their needs itself.
 *
 * Now it reads what the full rail reads:
 * - **Needs you**: the backend's one count (D-49), the bell's and the
 *   badge's, a sign-in or This Mac included; its button opens Home, where
 *   each one is listed with its next step.
 * - **Open now**: every open session, AI Juggler's and the ones other apps
 *   run, by place in Open now's order (D-47), each as its app's icon with
 *   the profile's dot (D-38), its state's dot, and `!` when it needs you.
 *   Its name says the state first, then the title and where it runs.
 * - **All clear** only when nothing needs you, nothing is open and the
 *   computer is calm (`RailCalm`).
 */
import { useId, useMemo, useRef, type JSX } from "react";
import { PlaceMark, ProfileBadge } from "./PlaceMarks";
import { ToolMark } from "./ToolMark";
import { providerDisplayName } from "../lib/providers";
import { RailCalm } from "./RailCalm";
import { useActionHost } from "./ItemMenu";
import { usePrimary, whereWords } from "./PlaceRows";
import { ariaSummary } from "./row-details";
import { useBackendNeeds } from "../lib/needs-you";
import { useRoving } from "../lib/roving";
import { useShownConversations } from "../lib/shown-apps";
import { useSeen } from "../lib/seen";
import { useRailOrder } from "../lib/rail-order-store";
import { arrangeGroups, rankOf, sortRows } from "../lib/rail-order";
import { groupItems } from "../lib/rail-places";
import { STATE_TONE, itemsOf, needWord, openNow, stateWord, type RailItem } from "../lib/rail-items";
import { isSnoozed, type SnoozeState } from "../lib/railview";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

/** "4 things need you", as the bell says it. */
export function needsYouWords(count: number): string {
  return `${count} ${count === 1 ? "thing needs" : "things need"} you`;
}

/** The open items in Open now's order: places as arranged, rows in them
 * as arranged (pins first), Latest activity's rows by when they started
 * (the folded column holds still). */
export function foldedOrder(items: RailItem[], mode: "yours" | "latest" | "name", places: readonly string[], rows: readonly string[]): RailItem[] {
  const rank = rankOf(rows);
  return arrangeGroups(groupItems(items.filter(openNow), "place"), mode, places).flatMap((group) => sortRows(group.items, mode === "name" ? "name" : "yours", rank));
}

export function FoldedRail({
  sessions,
  projects = [],
  snooze = {},
  now = Date.now(),
  selectedId,
  onSelect,
}: {
  sessions: InstanceInfo[];
  projects?: ProjectInfo[];
  snooze?: SnoozeState;
  now?: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
}): JSX.Element {
  const needs = useBackendNeeds();
  const { conversations } = useShownConversations();
  const seen = useSeen();
  const order = useRailOrder();
  const host = useActionHost();
  const pins = useMemo(() => new Set(order.pins), [order.pins]);
  const items = useMemo(
    () => itemsOf({ sessions, conversations, projects, seen, snoozed: (row) => isSnoozed(row, snooze, now), pins, needOf: needs.needOf }),
    [sessions, conversations, projects, seen, snooze, now, pins, needs.needOf],
  );
  const open = foldedOrder(items, order.mode, order.groups.place, order.rows);
  const count = needs.count;
  // F11: one stop on the Tab path; ↑↓ move through the icons.
  const root = useRef<HTMLDivElement | null>(null);
  useRoving(root);
  return (
    <div ref={root} className="flex flex-col items-center gap-1 pt-1" data-testid="rail-folded">
      {count > 0 && (
        <button
          type="button"
          data-rail-nav
          data-testid="rail-folded-needs-you"
          onClick={() => host.openHome?.()}
          aria-label={`Needs you: ${needsYouWords(count)}. Open Home`}
          title={`${needsYouWords(count)} — open Home`}
          className="mb-1 flex min-h-6 items-center gap-0.5 rounded-full border border-warning/40 bg-warning-soft px-1.5 py-0.5 text-label font-bold text-warning outline-none transition-colors hover:bg-warning/20 focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <span aria-hidden="true">!</span>
          <span className="tabular-nums">{count}</span>
        </button>
      )}
      {open.map((item) => (
        <FoldedRow key={item.key} item={item} selected={selectedId === (item.session?.id ?? item.conversation?.id)} onSelect={onSelect} />
      ))}
      {count === 0 && open.length === 0 && !needs.loading && <RailCalm />}
    </div>
  );
}

/** One open session as its icon: the app's with the profile's dot, its
 * state's dot, `!` when it needs you; the words in its name and tooltip. */
function FoldedRow({ item, selected, onSelect }: { item: RailItem; selected: boolean; onSelect: (id: string) => void }): JSX.Element {
  const primary = usePrimary(item, onSelect);
  const describedId = useId();
  const needs = item.need !== null;
  const word = needs ? needWord(item) : stateWord(item.state, item.probably);
  // AI Juggler's own sessions: the tool and account they run on, as the
  // place's app and profile name the others.
  const where = item.session ? [item.project, providerDisplayName(item.tool), item.accountLabel].filter(Boolean).join(" · ") : whereWords(item);
  const tone = needs ? STATE_TONE.asks_you : STATE_TONE[item.state];
  // AI Juggler's own sessions keep their fuller summary (I-01..I-11).
  const more = item.session ? ariaSummary(item.session) : where;
  return (
    <button
      type="button"
      data-rail-item={item.key}
      data-instance-id={item.session?.id}
      data-conversation-id={item.conversation?.id}
      aria-current={selected ? "true" : undefined}
      data-rail-selected={selected ? "true" : undefined}
      aria-describedby={describedId}
      onClick={primary}
      className={
        "group relative flex size-9 shrink-0 items-center justify-center rounded-lg outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (selected ? "bg-surface" : "hover:bg-surface")
      }
    >
      <span id={describedId} hidden>
        {more}
      </span>
      {/* WCAG 2.5.3: an icon-only button's name, in words, state first. */}
      <span className="sr-only">
        {word}, {item.title}, {where}
      </span>
      <span aria-hidden="true" className="flex">
        {item.session ? <OwnMark tool={item.tool} account={item.accountLabel} /> : <PlaceMark place={item.place} profile={item.accountLabel} size={22} />}
      </span>
      <span aria-hidden="true" data-state-dot={item.state} className={"absolute bottom-0.5 left-0.5 size-2 rounded-full ring-2 ring-canvas " + tone.dot} />
      {needs && (
        <span aria-hidden="true" data-testid="rail-folded-needs" className="absolute right-0 top-0 flex size-3.5 items-center justify-center rounded-full bg-warning text-label font-bold text-black">
          !
        </span>
      )}
      {/* Hover and focus: the words, never a bare icon. */}
      <span aria-hidden="true" className="pointer-events-none absolute left-full top-0 z-30 ml-2 hidden w-56 rounded-lg border border-hairline bg-surface p-2.5 text-left shadow-card group-hover:block group-focus-visible:block">
        <span className="block truncate text-meta font-medium text-ink">{item.title}</span>
        <span className={"block text-label font-semibold " + tone.text}>{word}</span>
        <span className="mt-0.5 block text-label leading-relaxed text-muted">{more}</span>
      </span>
    </button>
  );
}

/** An AI Juggler session's mark: its tool's, with its account's dot at
 * the corner where a place's mark has its profile's. */
function OwnMark({ tool, account }: { tool: string; account: string }): JSX.Element {
  return (
    <span className="relative inline-flex size-[22px] shrink-0 items-center justify-center rounded-md bg-surface-3" data-testid="folded-tool-mark">
      <ToolMark tool={tool} size={16} />
      <span className="absolute -bottom-0.5 -right-0.5 flex">
        <ProfileBadge name={account} size={7} ring="var(--color-canvas)" />
      </span>
    </span>
  );
}
