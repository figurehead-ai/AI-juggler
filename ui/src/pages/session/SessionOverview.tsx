/**
 * AJ-E2108 — a session page's Overview: its conversation, filling the
 * height under the header and scrolling inside itself, and beside it a
 * fixed column: where it runs, its account's runway and where it could
 * continue. The same for a session AI Juggler runs and one another app
 * runs. Loaded with the page, never at start; the runways and the
 * profiles come from one shared read (data.ts), not one per page.
 */
import { SwitchAccountButton } from "../../components/SwitchAccountButton";
import { useSwitchNotYet } from "../../lib/switch-not-yet";
import type { JSX } from "react";
import type { SwitchBackSuggestionData } from "../../lib/ipc/switchBack";
import { accountDisplay, accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { sessionAccountWords } from "./account-words";
import { LastMessages } from "./LastMessages";
import { ConversationSwitcher } from "./ConversationSwitcher";
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
  /** Opens the switch screen from this session (AI Juggler's or another
   * app's alike, AJ-E2108). */
  onPick?: (recipeId: string, label: string) => void;
  onCarryIn?: () => void;
  /** rc21: why Continue in AI Juggler can't run (its tool won't open it
   * by its id), or null. */
  carryInWhy?: string | null;
  switchBack?: SwitchBackSuggestionData | null;
  onSwitchBack?: () => void;
};

export function SessionOverview(props: OverviewProps): JSX.Element {
  const { model } = props;
  const accounts = useSubscriptions();
  const displays = useRunways();
  const profiles = useProfiles();
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  const named = sessionAccountWords(accounts, account);
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId);
  const switchNotYet = useSwitchNotYet(model.sessionId);
  return (
    <SessionColumns
      // 0.3.4: beside it, on its left, its subagents and the sessions it
      // started, one click to each (a paired Mac's stay there).
      main={
        props.remoteMac ? (
          <LastMessages model={model} visible={props.visible} remoteMac={props.remoteMac} onGoTo={props.onGoTo} />
        ) : (
          <ConversationSwitcher model={model} visible={props.visible} own={<LastMessages model={model} visible={props.visible} onGoTo={props.onGoTo} />} />
        )
      }
      // D-143 (PAT-SW): switching, always one visible click, first in the column.
      lead={props.remoteMac ? undefined : <SwitchAccountButton switchKey={model.switchSource} accountId={model.profileId} why={switchNotYet} />}
      side={
        <>
          <WhereItRuns model={model} outside={props.outside ?? false} accountWords={named.where}
            accountId={account?.state === "signed_in" ? (account.account_id ?? null) : null} />
          <Runway display={display} title={named.runway} profile={profiles.find((p) => p.id === model.profileId) ?? null} />
          {!props.remoteMac && (
            <ContinueOn
              model={model}
              accounts={accounts ?? []}
              displays={displays}
              onPick={props.onPick}
              onCarryIn={props.onCarryIn}
              carryInWhy={props.carryInWhy ?? null}
              switchBack={props.switchBack}
              onSwitchBack={props.onSwitchBack}
            />
          )}
        </>
      }
    />
  );
}
