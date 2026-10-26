/**
 * AJ-E2108 — a session page's Overview: its conversation, filling the
 * height under the header and scrolling inside itself, and beside it a
 * fixed column: where it runs, its account's runway and where it could
 * continue. The same for a session AI Juggler runs and one another app
 * runs. Loaded with the page, never at start; the runways and the
 * profiles' names come from one shared read (data.ts), not one per page.
 */
import { useMemo, type JSX } from "react";
import type { SwitchBackSuggestionData } from "../../lib/bridge";
import { accountDisplay, accountLine, accountName, accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { LastMessages } from "./LastMessages";
import { ContinueOn } from "./ContinueOn";
import { Runway } from "./Runway";
import { SessionColumns } from "./SessionColumns";
import { WhereItRuns } from "./WhereItRuns";
import { useProfiles, useRunways } from "./data";
import type { SessionModel } from "./model";

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

export function SessionOverview(props: OverviewProps): JSX.Element {
  const { model } = props;
  const accounts = useSubscriptions();
  const displays = useRunways();
  const profiles = useProfiles();
  const labels = useMemo(() => new Map(profiles.map((p) => [p.id, p.label])), [profiles]);
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId);
  return (
    <SessionColumns
      main={<LastMessages model={model} visible={props.visible} remoteMac={props.remoteMac} onGoTo={props.onGoTo} />}
      side={
        <>
          <WhereItRuns model={model} outside={props.outside ?? false} accountWords={account ? [accountName(account), accountLine(account)].filter(Boolean).join(" · ") : null} />
          <Runway display={display} title={account && account.state === "signed_in" ? accountName(account) : null} profile={profiles.find((p) => p.id === model.profileId) ?? null} />
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
        </>
      }
    />
  );
}
