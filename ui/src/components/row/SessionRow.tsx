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
 *   isn't repeated; in the rail the dot alone leads, under the App's
 *   header and its coloured edge.
 * - **Its state** ends the second line, in its colour and words (one
 *   vocabulary, AJ-E2104 Req.3: why it needs you when it does), never
 *   colour alone.
 * - **Its project** opens the project's page; left out on that page.
 * - **When** it was last active (or last looked at), never cut.
 *
 * Densities, never fields merged per screen (relations-map "One list"):
 * `cozy` and `compact` are the rail's (AJ-E403: Compact keeps the state's
 * word), `list` a page's or Home's (body text, the App icon, the tools in
 * a gutter of their own). A click opens its page here; ↗ goes to its app.
 */
import type { JSX, ReactNode } from "react";
import { AppIcon } from "../AppIcon";
import { lowLeftWords } from "../indicator";
import { PinMark } from "../RailProjectRows";
import type { Arrangeable } from "../RowTools";
import { shortHeavy, useHeavyNote } from "../../lib/machine";
import { STATE_TONE, needWord, type RailItem } from "../../lib/rail-items";
import { agoWords } from "../../lib/time-words";
import type { AppRef } from "../../lib/app-of";
import { RowShell } from "./RowShell";
import { PlaceLine, RowProject, RowSubagents, appWords } from "./RowParts";

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
  /** The title worked out for it (a list's: what was last asked). */
  title?: string;
  /** When, in words (default: last looked at, else last active). */
  when?: string;
  /** The app its icon shows, when the caller knows better (D-38: a
   * session that ended in the Claude app is the Claude app's). */
  app?: AppRef;
  /** The rail: the App's colour on the row's left edge. */
  edge?: string | null;
  arrange?: Arrangeable | null;
  /** The rail's Your order: the drag handle and the pin. */
  tools?: boolean;
  /** "Subagents" is on in the View: the row says how many it runs. */
  subagents?: boolean;
  /** Beside its state (History's "Archived"). */
  trailing?: ReactNode;
  /** In place of its state word, where every row of the list shares that
   * state (Home's Pick up: all ended, so its Mark reviewed stands there and
   * the title keeps its line; the 0.4 review). */
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

/** When it was last looked at or did something, in words. */
export function rowWhen(item: RailItem, now: number): string {
  if (item.viewedMs !== null) return `viewed ${agoWords(item.viewedMs, now)}`;
  return item.lastMs > 0 ? agoWords(item.lastMs, now) : "";
}

/** Its state, in words and colour: why it needs you when it does (amber,
 * D-47), else its state. */
export function rowState(item: RailItem): { word: string; text: string; dot: string } {
  if (item.need !== null) return { word: needWord(item), text: "text-warning", dot: "bg-warning" };
  return { word: needWord(item), ...STATE_TONE[item.state] };
}

export function SessionRow(props: SessionRowProps): JSX.Element {
  const { item, density, now, within = {}, subagents } = props;
  const title = props.title ?? item.title;
  const needs = item.need !== null;
  const state = rowState(item);
  const low = item.session ? lowLeftWords(item.session) : null;
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
  const label = [state.word, title, pinned ? "pinned" : null, line, on, also, low, heavyNote ? `working your Mac hard: ${heavyNote}` : null, helpers].filter(Boolean).join(", ");
  const dot = (
    <span aria-hidden="true" data-state-dot={item.state} className={"size-1.5 shrink-0 rounded-full " + state.dot + (item.state === "working" && !needs ? " animate-breathe" : "")} />
  );
  const exception = low ? (
    <span className="ml-auto shrink-0 text-label text-warning">{low}</span>
  ) : heavy ? (
    <span className="ml-auto shrink-0 text-label text-warning" data-testid="rail-heavy">
      {heavy}
    </span>
  ) : null;
  const shell = { item, selected: props.selected ?? false, onSelect: props.onSelect, label, arrange: props.arrange, tools: props.tools, below: props.below, slim: props.slim, menuKind: props.menuKind, data: props.data, buttonData: props.buttonData };

  if (density === "list") {
    return (
      <RowShell
        {...shell}
        layout="list"
        shape=""
        className="gap-3.5 py-2.5 pl-4"
        data={{ "data-testid": "page-session-item", ...props.data }}
        buttonData={{ "data-testid": "page-session-row", "data-outside": item.session ? undefined : "true", "data-live": item.live ? "true" : undefined, ...props.buttonData }}
      >
        <span aria-hidden="true" className="relative flex shrink-0">
          <AppIcon {...(props.app ?? { appId: item.place.appId, appPid: item.place.appPid, name: item.place.app })} size={24} />
          <span data-state-dot={item.state} className={"absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full ring-2 ring-surface " + state.dot} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          {/* Not hidden whole: what the list adds there is a control. */}
          <span className="flex min-w-0 items-center gap-1.5">
            <span aria-hidden="true" className="truncate text-body font-medium text-ink" data-testid="page-session-title">
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
            {props.stateSlot ?? (
              <span aria-hidden="true" className={"shrink-0 text-meta leading-[18px] " + (needs ? "font-semibold " : "font-medium ") + state.text} data-testid="page-session-state">
                {state.word}
              </span>
            )}
          </span>
        </span>
      </RowShell>
    );
  }
  const edge = { borderLeftColor: props.edge ?? "var(--color-hairline)" };
  if (density === "compact") {
    // One line: the state's dot, the title, then an exception (low
    // runway, heavy load) or when, and the state word (AJ-E403: the
    // state's words never go).
    return (
      <RowShell {...shell} indent shape="rounded-r-lg" className="h-[30px] gap-2 border-l-2 pl-2.5" style={edge}>
        {dot}
        <span aria-hidden="true" className="flex min-w-0 flex-1 items-center gap-1.5">
          <span className="truncate text-meta font-semibold text-ink">{title}</span>
          {pinned && <PinMark />}
          <span className="ml-auto shrink-0 text-label text-muted">{low ? <span className="text-warning">{low}</span> : heavy ? <span className="text-warning" data-testid="rail-heavy">{heavy}</span> : when}</span>
          <span className={"shrink-0 text-label font-medium " + state.text} data-testid="state-word">
            {state.word}
          </span>
        </span>
      </RowShell>
    );
  }
  return (
    <RowShell {...shell} indent shape="rounded-r-lg" className="h-[44px] gap-2 border-l-2 py-1 pl-2.5" style={edge}>
      {dot}
      {/* The title has its line to itself; its state ends the line under
          it, after the project and the time (the review of 0.1.4). */}
      <span className="flex min-w-0 flex-1 flex-col gap-px">
        <span aria-hidden="true" className="flex min-w-0 items-center gap-1">
          <span className="truncate text-meta font-semibold text-ink">{title}</span>
          {pinned && <PinMark />}
          {exception}
        </span>
        <span className="flex min-w-0 items-center gap-2 text-label text-muted">
          <span className="flex min-w-0 flex-1 items-center">
            <RowProject item={projectShown ? item : { ...item, project: null }} rest={when} whole />
            {also && <span aria-hidden="true" className="shrink-0 whitespace-pre" data-testid="row-also"> · {also}</span>}
            {helpers && <RowSubagents item={item} words={helpers} onSelect={props.onSelect} />}
          </span>
          <span aria-hidden="true" className={"shrink-0 text-label leading-[14px] " + (needs ? "font-semibold " : "font-medium ") + state.text} data-testid="state-word">
            {state.word}
          </span>
        </span>
      </span>
    </RowShell>
  );
}
