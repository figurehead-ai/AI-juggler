/**
 * AJ-E2108 — a session page's Overview: its conversation, filling the
 * height under the header and scrolling inside itself, and beside it a
 * fixed column: where it runs, its account's runway and where it could
 * continue. The same for a session AI Juggler runs and one another app
 * runs. Loaded with the page, never at start; the runways and the
 * profiles' names come from one shared read (data.ts), not one per page.
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Card } from "../../components/primitives";
import type { SwitchBackSuggestionData } from "../../lib/bridge";
import { accountDisplay, accountLine, accountName, accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { shortPath } from "../../screens/AccountManage";
import { CardTitle } from "./CardTitle";
import { LastMessages } from "./LastMessages";
import { ContinueOn } from "./ContinueOn";
import { Runway } from "./Runway";
import { useProfiles, useRunways } from "./data";
import { appWords, type SessionModel } from "./model";

export type OverviewProps = {
  model: SessionModel;
  /** While the Overview is the shown tab (last messages wait for it). */
  visible: boolean;
  /** A paired Mac's session: its messages stay there. */
  remoteMac?: string | null;
  /** Started outside AI Juggler: said once, quietly, under where it runs. */
  outside?: boolean;
  onGoTo?: () => void;
  onPick?: (recipeId: string, label: string) => void;
  onCarryIn?: () => void;
  switchBack?: SwitchBackSuggestionData | null;
  onSwitchBack?: () => void;
};

/** Whether a box that scrolls has more below what shows (a short window:
 * the side column's last card under the fold says so with a fade). */
function useMoreBelow(): [React.RefObject<HTMLDivElement | null>, boolean, () => void] {
  const box = useRef<HTMLDivElement | null>(null);
  const [more, setMore] = useState(false);
  const measure = () => {
    const el = box.current;
    if (el) setMore(el.scrollHeight - el.clientHeight - el.scrollTop > 4);
  };
  useEffect(() => {
    measure();
    const el = box.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(measure);
    watch.observe(el);
    for (const child of el.children) watch.observe(child);
    return () => watch.disconnect();
  });
  return [box, more, measure];
}

export function SessionOverview(props: OverviewProps): JSX.Element {
  const { model } = props;
  const accounts = useSubscriptions();
  const displays = useRunways();
  const profiles = useProfiles();
  const labels = useMemo(() => new Map(profiles.map((p) => [p.id, p.label])), [profiles]);
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId);
  const [side, moreBelow, onSideScroll] = useMoreBelow();
  return (
    <div className="@container flex min-h-0 flex-1 flex-col" data-testid="session-overview">
      <div className="mx-auto grid w-full max-w-[1440px] min-h-0 flex-1 grid-cols-1 gap-4 @4xl:grid-cols-[minmax(0,1fr)_340px] @4xl:grid-rows-[minmax(0,1fr)]">
        <div className="flex min-h-[28rem] min-w-0 flex-col @4xl:min-h-0">
          <LastMessages model={model} visible={props.visible} remoteMac={props.remoteMac} onGoTo={props.onGoTo} />
        </div>
        <div
          ref={side}
          onScroll={onSideScroll}
          className={
            "flex min-w-0 flex-col gap-4 rounded-card outline-none focus-visible:ring-1 focus-visible:ring-accent/60 @4xl:min-h-0 @4xl:overflow-y-auto " +
            (moreBelow ? "@4xl:[mask-image:linear-gradient(to_bottom,black_calc(100%-48px),transparent)]" : "")
          }
          data-testid="session-side"
          data-more-below={moreBelow ? "true" : undefined}
          tabIndex={0}
          role="region"
          aria-label="About this session"
        >
          <WhereItRuns model={model} outside={props.outside ?? false} accountWords={account ? [accountName(account), accountLine(account)].filter(Boolean).join(" · ") : null} />
          <Runway display={display} title={account && account.state === "signed_in" ? accountName(account) : null} />
          {!props.remoteMac && (
            <ContinueOn
              model={model}
              accounts={accounts ?? []}
              labels={labels}
              displays={displays}
              onPick={props.onPick}
              onCarryIn={props.onCarryIn}
              switchBack={props.switchBack}
              onSwitchBack={props.onSwitchBack}
            />
          )}
        </div>
      </div>
    </div>
  );
}

/** App, window, profile, account and model: only the ones it knows. */
function WhereItRuns({ model, accountWords, outside }: { model: SessionModel; accountWords: string | null; outside: boolean }): JSX.Element {
  const place = model.places[0];
  const app = appWords(place, model.tool) ?? (model.sessionId && model.live ? "AI Juggler's terminal" : null);
  const rows: [string, string, boolean?][] = [
    ["App", app ? app.replace(/^the /, "") : model.live ? "Not known" : "Not open now"],
    ...(place?.instance ? ([["Window", place.label]] as [string, string][]) : []),
    ["Profile", model.profileLabel],
    ...(accountWords ? ([["Account", accountWords]] as [string, string][]) : []),
    ...(model.model ? ([["Model", model.model, true]] as [string, string, boolean][]) : []),
    ...(model.cwd ? ([["Folder", shortPath(model.cwd), true]] as [string, string, boolean][]) : []),
  ];
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="where-it-runs">
      <CardTitle>Where it runs</CardTitle>
      <dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-4 gap-y-2 text-meta">
        {rows.map(([label, value, mono]) => (
          <div key={label} className="contents">
            <dt className="text-muted">{label}</dt>
            <dd className={"min-w-0 truncate text-ink " + (mono ? "font-mono text-meta" : "")} title={value}>
              {value}
            </dd>
          </div>
        ))}
      </dl>
      {model.places.length > 1 && (
        <p className="text-meta text-muted" data-testid="where-also">
          Also open in {model.places.slice(1).map((p) => p.label || p.app).join(", ")}.
        </p>
      )}
      {outside && (
        <p className="border-t border-hairline pt-3 text-label leading-[1.5] text-muted" data-testid="where-outside">
          Started <span data-testid="outside-chip">outside AI Juggler</span>: shown here to read and follow. AI Juggler never stops or changes it.
        </p>
      )}
    </Card>
  );
}
