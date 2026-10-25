/**
 * The Overview of a session AI Juggler runs in its terminal: the same page
 * as any session's (the review of 0.1.5: a terminal with a fold under
 * it). Its terminal is the main pane, since it is the conversation; beside
 * it the side column: where it runs (with its project and its details),
 * its account's runway, and Continue on another account.
 */
import { useMemo, type JSX, type ReactNode } from "react";
import type { InstanceInfo, ProjectInfo, SwitchBackSuggestionData } from "../../lib/bridge";
import { useConversations } from "../../lib/conversations";
import { accountDisplay, accountLine, accountName, accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { HandoverSection } from "../../components/HandoverSection";
import { Runway } from "./Runway";
import { SessionColumns } from "./SessionColumns";
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
}: {
  session: InstanceInfo;
  project: ProjectInfo | null;
  onOpenProject: (id: string) => void;
  /** Its terminal (and, ended, the line that says its scrollback was replayed). */
  terminal: ReactNode;
  /** Its details (when it ended, its notes, its launch settings and ids). */
  details: ReactNode;
  switchBackSuggestions?: SwitchBackSuggestionData[];
  onSwitchBack?: (sessionId: string) => void;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
}): JSX.Element {
  const { all } = useConversations();
  const accounts = useSubscriptions();
  const displays = useRunways();
  const profiles = useProfiles();
  const seen = all.find((c) => c.session_id === session.id) ?? all.find((c) => c.account_id === session.account_id && c.id === session.conversation_id);
  // The same object while nothing it shows has changed.
  const json = JSON.stringify(fromInstance(session, seen));
  const model = useMemo(() => JSON.parse(json) as ReturnType<typeof fromInstance>, [json]);
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId);
  return (
    <SessionColumns
      testId="own-session-overview"
      main={terminal}
      side={
        <>
          <WhereItRuns
            model={model}
            outside={false}
            accountWords={account ? [accountName(account), accountLine(account)].filter(Boolean).join(" · ") : null}
            project={project ? { name: project.name, onOpen: () => onOpenProject(project.id) } : null}
          >
            {details}
          </WhereItRuns>
          <Runway display={display} title={account && account.state === "signed_in" ? accountName(account) : null} profile={profiles.find((p) => p.id === model.profileId) ?? null} />
          <HandoverSection session={session} switchBackSuggestions={switchBackSuggestions} onSwitchBack={onSwitchBack} onHandover={onHandover} />
        </>
      }
    />
  );
}
