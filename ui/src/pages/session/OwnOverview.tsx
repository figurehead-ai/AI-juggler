/**
 * AJ-E2108 — the Overview of a session AI Juggler started: the same
 * Overview as any session's, with its conversation as the tools' files
 * show it (where it's open, when it was viewed) and the switch screen
 * behind Continue on another account.
 */
import type { JSX } from "react";
import type { InstanceInfo, SwitchBackSuggestionData } from "../../lib/bridge";
import { useConversations } from "../../lib/conversations";
import { SessionOverview } from "./SessionOverview";
import { fromInstance } from "./model";

export function OwnSessionOverview({
  session,
  visible,
  onHandover,
  switchBackSuggestions = [],
  onSwitchBack,
}: {
  session: InstanceInfo;
  visible: boolean;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
  switchBackSuggestions?: SwitchBackSuggestionData[];
  onSwitchBack?: (sessionId: string) => void;
}): JSX.Element {
  const { all } = useConversations();
  const seen =
    all.find((c) => c.session_id === session.id) ??
    all.find((c) => c.account_id === session.account_id && c.id === session.conversation_id);
  return (
    <SessionOverview
      model={fromInstance(session, seen)}
      visible={visible}
      onPick={(recipeId, label) => onHandover(session.id, recipeId, label)}
      switchBack={switchBackSuggestions.find((suggestion) => suggestion.session_id === session.id) ?? null}
      onSwitchBack={onSwitchBack ? () => onSwitchBack(session.id) : undefined}
    />
  );
}
