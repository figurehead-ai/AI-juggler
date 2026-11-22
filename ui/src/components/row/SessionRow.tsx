/**
 * E31 (AJ-E3103, D-47, D-36 with D-62's words) — one session's row, the
 * one everywhere a session shows: the rail, Home and every page's lists.
 * Wherever it is seen it says the same things in the same places:
 *
 *   [App icon · state dot]  Title ·························· [pin][low or heavy]
 *                           project · ● App · when ··················· State
 *
 * - **Its App**: the app's own icon (D-38) with the state's dot at its
 *   corner, and in the place line the App's one colour beside its name
 *   (AJ-E3206: colour is the Apps'). Where the App is already said — a
 *   rail group under its App's header, an App's page — the App's name
 *   isn't repeated; in the rail the state's mark alone leads, under the
 *   App's header (D-117: no coloured edge; the App is said once).
 * - **Its state** ends the second line, in its level's colour and words (one
 *   vocabulary, AJ-E2104 Req.3: why it needs you when it does), never
 *   colour alone.
 * - **Its project** opens the project's page; left out on that page.
 * - **When** it was last active (or last looked at), never cut.
 *
 * Densities, never fields merged per screen (relations-map "One list"):
 * `cozy` and `compact` are the rail's, one line each (D-118's mock, round
 * 3: the title, then its state at the right; where and when in its
 * tooltip and name; AJ-E403: Compact keeps the state's word and adds the
 * time), `list` a page's or Home's (body text, the App icon, the tools in
 * a gutter of their own). A click opens its page here; ↗ goes to its app.
 */
import type { JSX, ReactNode } from "react";
import { AppIcon } from "../AppIcon";
import { lowLeftWords } from "../indicator";
import { PinMark } from "../RailProjectRows";
import type { Arrangeable } from "../RowTools";
import { shortHeavy, useHeavyNote } from "../../lib/machine";
import { STATE_TONE, needWord, type RailItem } from "../../lib/rail-items";
import { itemLevel, levelTone, type AttentionLevel } from "../../lib/attention";
import { agoWords } from "../../lib/time-words";
import { parkedWords } from "../../lib/parked";
import type { AppRef } from "../../lib/app-of";
import { RowShell } from "./RowShell";
import { needWait } from "../../lib/waiting";
import { PlaceLine, RowSubagents, appWords } from "./RowParts";

export type RowDensity = "cozy" | "compact" | "list";

export interface SessionRowProps {
  item: RailItem;
  density: RowDensity;
  now: number;
  /** Opens a session's page (one another app runs opens through the host). */
  onSelect: (id: string) => void;
  selected?: boolean;
  /** What the place it shows in already says: its App (a rail group, an
   * App's page), its project (a project's page). */
  within?: { app?: boolean; project?: boolean };
  /** The title worked out for it (Home's: told apart from the rows beside it). */
  title?: string;
  /** When, in words (default: last looked at, else last active). */
  when?: string;
  /** The app its icon shows, when the caller knows better (D-38: a
   * session that ended in the Claude app is the Claude app's). */
  app?: AppRef;
  arrange?: Arrangeable | null;
  /** The rail's Your order: the drag handle and the pin. */
  tools?: boolean;
  /** "Subagents" is on in the View: the row says how many it runs. */
  subagents?: boolean;
  /** 0.3.9: how deep it sits under the session it was started from. */
  depth?: number;
  /** That session's title, for a screen reader. */
  parent?: string | null;
  /** Beside its state (History's "Archived"). */
  trailing?: ReactNode;
  /** In place of its state word, where every row of the list shares that
   * state (Home's Pick up: all ended, so its Mark reviewed stands there and
   * the title keeps its line; the 0.4 review); `null`, nothing there (its
   * section's heading says the state). */
  stateSlot?: ReactNode;
  /** Under it (History's matching words). */
  below?: ReactNode;
  data?: Record<string, string | undefined>;
  buttonData?: Record<string, string | undefined>;
  /** A list of ended rows: the gutter holds the ⋯ alone (Home). */
  slim?: boolean;
  /** Which menu it has (a list's ended session: its history entry's). */
  menuKind?: "session" | "history" | "conversation";
}

/** How long it has waited on you (AJ-E3307: Home's and the bell's rule,
 * `lib/waiting`), else when it was last looked at or did something. */
export function rowWhen(item: RailItem, now: number): string {
  const waits = needWait(item, now);
  if (waits) return waits;
  if (item.viewedMs !== null) return `viewed ${agoWords(item.viewedMs, now)}`;
  return item.lastMs > 0 ? agoWords(item.lastMs, now) : "";
}

/** Its state, in words and colour: why it needs you when it does, in its
 * level's colour (D-117: amber L2, red L3; a review, L1, is muted with its
 * ✓), else its state (calm); parked, as its rail row and its page's header
 * say it ("Parked 3m", "Parked · ready"; AJ-E2312 1), given `now`. */
export function rowState(item: RailItem, now?: number): { word: string; text: string; dot: string; level: AttentionLevel } {
  const level = itemLevel(item);
  if (level >= 2) return { word: needWord(item), ...levelTone(level), level };
  const parked = item.session?.parked && now !== undefined ? parkedWords(item.session, now) : null;
  return { word: parked?.chip ?? needWord(item), ...STATE_TONE[item.state], level };
}

export function StateMark({ item, level, dot }: { item: RailItem; level: AttentionLevel; dot: string }): JSX.Element {
  if (level >= 2) return <span aria-hidden="true" data-state-dot={item.state} data-level={level} data-probably={item.probably ? "true" : undefined} className="level-mark" />;
  if (item.need === "review") return <span aria-hidden="true" data-state-dot={item.state} className="flex w-1.5 shrink-0 justify-center overflow-visible text-label leading-none text-success">✓</span>;
  return <span aria-hidden="true" data-state-dot={item.state} className={"size-1.5 shrink-0 rounded-full " + dot} />;
}

export function SessionRow(props: SessionRowProps): JSX.Element {
  const { item, density, now, within = {}, subagents } = props;
  const title = props.title ?? item.title;
  const state = rowState(item, now);
  // AJ-E3603: a row its limit stopped says "Limit reached"; "0% left"
  // beside it said it twice and cut its title to "Tidy the …".
  const low = item.session && item.need !== "limit" ? lowLeftWords(item.session) : null;
  // D-118 (one quiet mark per need, the 0.5 review of the rail): a row's
  // low room is a fact, muted at every level. The need's one loud copy is
  // its Needs you row; here the row's dot and its word in ink say it (D-47
  // as amended by D-117), so a row its limit stopped no longer reads
  // "0% left" in amber beside its Needs you row. The Account's own runway
  // line keeps the Account's level (red when out with live work). So is
  // a heavy load (AJ-E3603): its loud copy is the Needs you "heavy" row.
  const lowLevel: AttentionLevel = 0;
  const lowTone = "text-muted";
  const heavyNote = useHeavyNote(item.session?.id ?? "");
  const heavy = heavyNote ? shortHeavy(heavyNote) : null;
  const when = props.when ?? rowWhen(item, now);
  const appShown = !within.app;
  const projectShown = !within.project;
  const line = [projectShown ? item.project : null, density === "list" && appShown ? appWords(item) : null, when].filter(Boolean).join(" · ");
  const also = item.alsoIn.length > 0 ? `also open in ${item.alsoIn.join(" and ")}` : null;
  const helpers = subagents && item.subagents > 0 ? `+${item.subagents} ${item.subagents === 1 ? "agent" : "agents"}` : null;
  // AJ-E1502: a pinned row says so, as the Project lens's row does.
  const pinned = item.pinned;
  // AI Juggler's own session: the account it runs on too (its place is
  // AI Juggler, which names no profile), so a screen reader hears it.
  const on = item.session && density !== "list" ? `on ${item.accountLabel}` : null;
  const from = props.parent ? `started from ${props.parent}` : null;
  const label = [state.word, title, pinned ? "pinned" : null, from, line, on, also, low, heavyNote ? `working your Mac hard: ${heavyNote}` : null, helpers].filter(Boolean).join(", ");
  const dot = <StateMark item={item} level={state.level} dot={state.dot} />;
  // D-117 (amends D-47 point 2): in the rail a need keeps its place and
  // its word, in normal ink beside its level's mark; the loud copy is its
  // Needs you row. Titles are medium ink-2, the selected row semibold ink.
  const railWord = state.level >= 2 ? "text-ink-2" : "text-muted";
  const railTitle = props.selected ? "font-semibold text-ink" : "font-medium text-ink-2";
  const exception = low ? (
    <span className={"ml-auto shrink-0 text-label " + lowTone} data-level={lowLevel}>{low}</span>
  ) : heavy ? (
    <span className="ml-auto shrink-0 text-label text-muted" data-testid="rail-heavy">
      {heavy}
    </span>
  ) : null;
  const shell = { item, selected: props.selected ?? false, onSelect: props.onSelect, label, arrange: props.arrange, tools: props.tools, below: props.below, slim: props.slim, menuKind: props.menuKind, data: { "data-level": String(state.level), "data-need": item.need ?? undefined, "data-state": item.state, ...props.data }, buttonData: props.buttonData, depth: props.depth };

  if (density === "list") {
    return (
      <RowShell
        {...shell}
        layout="list"
        shape=""
        className="gap-3.5 py-2.5 pl-4"
        data={{ "data-testid": "page-session-item", ...shell.data }}
        buttonData={{ "data-testid": "page-session-row", "data-outside": item.session ? undefined : "true", "data-live": item.live ? "true" : undefined, ...props.buttonData }}
      >
        <span aria-hidden="true" className="relative flex shrink-0">
          <AppIcon {...(props.app ?? { appId: item.place.appId, appPid: item.place.appPid, name: item.place.app })} size={24} />
          <span data-state-dot={item.state} className={"absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full ring-2 ring-surface " + state.dot} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          {/* Not hidden whole: what the list adds there is a control. */}
          <span className="flex min-w-0 items-center gap-1.5">
            <span aria-hidden="true" className="truncate text-body font-medium text-ink" data-testid="page-session-title" title={title}>
              {title}
            </span>
            {pinned && (
              <span aria-hidden="true" className="flex">
                <PinMark />
              </span>
            )}
            {/* Its exception, then what this list adds (Home's Mark
                reviewed, History's Archived), at the title line's end:
                the place line keeps the row's width. */}
            {(exception || props.trailing) && (
              <span className="ml-auto flex shrink-0 items-center gap-2">
                {exception && <span aria-hidden="true">{exception}</span>}
                {props.trailing}
              </span>
            )}
          </span>
          <span className="flex min-w-0 items-start gap-3">
            <PlaceLine item={item} when={when} app={appShown} project={projectShown} />
            {props.stateSlot !== undefined ? props.stateSlot : (
              <span aria-hidden="true" className={"shrink-0 text-meta leading-[18px] " + (state.level >= 2 ? "font-semibold " : "font-medium ") + state.text} data-testid="page-session-state">
                {state.word}
              </span>
            )}
          </span>
        </span>
      </RowShell>
    );
  }
  // Where and when it was, for its tooltip (its name says them too).
  const hover = [title, line, also].filter(Boolean).join(" — ");
  if (density === "compact") {
    // One line: the state's dot, the title, then an exception (low
    // runway, heavy load) or when, and the state word (AJ-E403: the
    // state's words never go). AJ-E3603: the time shows only where the
    // row has room for it beside a readable title (a rail at its usual
    // width cut titles to "Refa…" to fit "waiting 3 h"); it stays in the
    // row's tooltip and name.
    return (
      <RowShell {...shell} indent shape="rounded-lg" className="h-[30px] gap-2 pl-2.5">
        {dot}
        <span aria-hidden="true" className="@container flex min-w-0 flex-1 items-center gap-1.5">
          <span className={"min-w-0 truncate text-meta " + railTitle} title={hover} data-testid="row-title">{title}</span>
          {pinned && <PinMark />}
          <span className="ml-auto shrink-0 text-label text-muted">{low ? <span className={lowTone} data-level={lowLevel}>{low}</span> : heavy ? <span className="text-muted" data-testid="rail-heavy">{heavy}</span> : when ? <span className="hidden @min-[16rem]:inline" data-testid="row-when">{when}</span> : null}</span>
          <span className={"shrink-0 text-label font-medium " + railWord} data-testid="state-word">
            {state.word}
          </span>
        </span>
      </RowShell>
    );
  }
  // D-118's mock (round 3): one line, the title then its state at the
  // right, so more of the list fits; where and when it was (project ·
  // time, and where else it is open) are in its tooltip and its name.
  return (
    <RowShell {...shell} indent shape="rounded-lg" className="h-[32px] gap-2 pl-2.5">
      {dot}
      <span className="flex min-w-0 flex-1 items-center gap-1.5">
        <span aria-hidden="true" className={"min-w-0 truncate text-meta " + railTitle} title={hover} data-testid="row-title">{title}</span>
        {pinned && (
          <span aria-hidden="true" className="flex">
            <PinMark />
          </span>
        )}
        <span className="ml-auto flex shrink-0 items-center gap-2">
          {exception && <span aria-hidden="true" className="flex">{exception}</span>}
          {helpers && <RowSubagents item={item} words={helpers} onSelect={props.onSelect} />}
          <span aria-hidden="true" className={"shrink-0 text-label font-medium " + railWord} data-testid="state-word">
            {state.word}
          </span>
        </span>
      </span>
    </RowShell>
  );
}
