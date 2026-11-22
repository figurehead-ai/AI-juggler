/**
 * The Overview of a session AI Juggler runs in its terminal: the same page
 * as any session's (the review of 0.1.5: a terminal with a fold under
 * it). Its terminal is the main pane, since it is the conversation; beside
 * it the side column: where it runs (with its project and its details),
 * its account's runway, and Continue on another account.
 */
import { SwitchAccountButton } from "../../components/SwitchAccountButton";
import { useSwitchNotYet } from "../../lib/switch-not-yet";
import { useMemo, type JSX, type ReactNode } from "react";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { SwitchBackSuggestionData } from "../../lib/ipc/switchBack";
import { useConversations } from "../../lib/conversations";
import { accountDisplay, accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { sessionAccountWords } from "./account-words";
import { HandoverSection } from "../../components/HandoverSection";
import { Runway } from "./Runway";
import { SessionColumns } from "./SessionColumns";
import { ConversationSwitcher } from "./ConversationSwitcher";
import { WhereItRuns } from "./WhereItRuns";
import { useProfiles, useRunways } from "./data";
import { fromInstance } from "./model";

export function OwnTerminalOverview({
  session,
  project,
  onOpenProject,
  terminal,
  details,
  switchBackSuggestions,
  onSwitchBack,
  onHandover,
  reportedModel = null,
}: {
  session: InstanceInfo;
  project: ProjectInfo | null;
  onOpenProject: (id: string) => void;
  /** Its terminal; ended, what it kept (SessionTerminal: the terminal replayed, else its conversation). */
  terminal: ReactNode;
  /** Its details (when it ended, its notes, its launch settings and ids). */
  details: ReactNode;
  switchBackSuggestions?: SwitchBackSuggestionData[];
  onSwitchBack?: (sessionId: string) => void;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
  /** A driven session's model as its tool reports it (its driver card's),
   * until its transcript says. */
  reportedModel?: string | null;
}): JSX.Element {
  const { all } = useConversations();
  const accounts = useSubscriptions();
  const displays = useRunways();
  const profiles = useProfiles();
  const seen = all.find((c) => c.session_id === session.id) ?? all.find((c) => c.account_id === session.account_id && c.id === session.conversation_id);
  // The same object while nothing it shows has changed.
  const json = JSON.stringify(fromInstance(session, seen, reportedModel));
  const model = useMemo(() => JSON.parse(json) as ReturnType<typeof fromInstance>, [json]);
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  const named = sessionAccountWords(accounts, account);
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId);
  const switchNotYet = useSwitchNotYet(session.id);
  return (
    <SessionColumns
      testId="own-session-overview"
      // 0.3.4: its subagents and the sessions it started, beside it on its
      // left, each one click away; the terminal stays as it is meanwhile.
      main={<ConversationSwitcher model={model} visible own={terminal} ownWords="Its terminal" />}
      // D-143 (PAT-SW): switching, always one visible click, first in the column.
      lead={<SwitchAccountButton switchKey={session.id} accountId={session.account_id} why={switchNotYet} />}
      side={
        <>
          <WhereItRuns
            model={model}
            outside={false}
            accountWords={named.where}
            accountId={account?.state === "signed_in" ? (account.account_id ?? null) : null}
            project={project ? { name: project.name, onOpen: () => onOpenProject(project.id) } : null}
          >
            {details}
          </WhereItRuns>
          <Runway display={display} title={named.runway} profile={profiles.find((p) => p.id === model.profileId) ?? null} />
          <HandoverSection session={session} switchBackSuggestions={switchBackSuggestions} onSwitchBack={onSwitchBack} onHandover={onHandover} />
        </>
      }
    />
  );
}
