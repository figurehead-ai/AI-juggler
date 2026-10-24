/**
 * AJ-E2108 — a session page's Overview: its last messages (two columns),
 * and beside them where it runs, its account's runway and where it could
 * continue. The same for a session AI Juggler runs and one another app
 * runs. Loaded with the page, never at start.
 */
import { useEffect, useMemo, useState, type JSX } from "react";
import { Card } from "../../components/primitives";
import { AccountRunway } from "../../components/AccountRunway";
import { accountsList, quotaDisplayAll, type AccountInfo, type QuotaDisplayData, type SwitchBackSuggestionData } from "../../lib/bridge";
import { accountDisplay, accountLine, accountName, accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { shortPath } from "../../screens/AccountManage";
import { LastMessages } from "./LastMessages";
import { ContinueOn } from "./ContinueOn";
import { appWords, type SessionModel } from "./model";

export type OverviewProps = {
  model: SessionModel;
  /** While the Overview is the shown tab (last messages wait for it). */
  visible: boolean;
  /** A paired Mac's session: its messages stay there. */
  remoteMac?: string | null;
  onPick?: (recipeId: string, label: string) => void;
  onCarryIn?: () => void;
  switchBack?: SwitchBackSuggestionData | null;
  onSwitchBack?: () => void;
};

export function SessionOverview(props: OverviewProps): JSX.Element {
  const { model } = props;
  const accounts = useSubscriptions();
  const [displays, setDisplays] = useState<QuotaDisplayData[]>([]);
  const [profiles, setProfiles] = useState<AccountInfo[]>([]);
  useEffect(() => {
    let alive = true;
    void quotaDisplayAll().then((rows) => alive && setDisplays(rows), () => null);
    void accountsList().then((rows) => alive && setProfiles(rows), () => null);
    return () => {
      alive = false;
    };
  }, []);
  const labels = useMemo(() => new Map(profiles.map((p) => [p.id, p.label])), [profiles]);
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  return (
    <div className="@container" data-testid="session-overview">
      <div className="grid grid-cols-1 gap-3 @3xl:grid-cols-3">
        <div className="flex min-w-0 flex-col gap-3 @3xl:col-span-2">
          <LastMessages model={model} visible={props.visible} remoteMac={props.remoteMac} />
        </div>
        <div className="flex min-w-0 flex-col gap-3">
          <WhereItRuns model={model} accountWords={account ? [accountName(account), accountLine(account)].filter(Boolean).join(" · ") : null} />
          <Card className="flex flex-col gap-2.5 p-4" data-testid="session-runway">
            <h3 className="text-[11px] font-medium uppercase tracking-wider text-muted">Runway{account && account.state === "signed_in" ? ` · ${accountName(account)}` : ""}</h3>
            <AccountRunway display={account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId)} />
          </Card>
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
function WhereItRuns({ model, accountWords }: { model: SessionModel; accountWords: string | null }): JSX.Element {
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
    <Card className="flex flex-col gap-2.5 p-4" data-testid="where-it-runs">
      <h3 className="text-[11px] font-medium uppercase tracking-wider text-muted">Where it runs</h3>
      <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5 text-[12px]">
        {rows.map(([label, value, mono]) => (
          <div key={label} className="contents">
            <dt className="text-muted">{label}</dt>
            <dd className={"min-w-0 truncate text-ink " + (mono ? "font-mono text-[11.5px]" : "")} title={value}>
              {value}
            </dd>
          </div>
        ))}
      </dl>
      {model.places.length > 1 && (
        <p className="text-[11px] text-muted" data-testid="where-also">
          Also open in {model.places.slice(1).map((p) => p.label || p.app).join(", ")}.
        </p>
      )}
    </Card>
  );
}
