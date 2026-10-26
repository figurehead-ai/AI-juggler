/**
 * AJ-E2108 — the Overview of a session AI Juggler started: the same
 * Overview as any session's, with its conversation as the tools' files
 * show it (where it's open, when it was viewed) and the switch screen
 * behind Continue on another account. Go to it from its question card
 * goes where you answer it: its app's window, or its own Terminal tab.
 */
import { useCallback, useMemo, type JSX } from "react";
import type { InstanceInfo, SwitchBackSuggestionData } from "../../lib/bridge";
import { useConversations } from "../../lib/conversations";
import { useActionHost } from "../../components/ItemMenu";
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
  const host = useActionHost();
  const { all } = useConversations();
  const seen =
    all.find((c) => c.session_id === session.id) ??
    all.find((c) => c.account_id === session.account_id && c.id === session.conversation_id);
  // The same object while nothing it shows has changed (refreshes hand
  // over new rows with the same values).
  const json = JSON.stringify(fromInstance(session, seen));
  const model = useMemo(() => JSON.parse(json) as ReturnType<typeof fromInstance>, [json]);
  const desktop = session.kind === "desktop";
  const goTo = useCallback(() => {
    if (desktop) void host.focusSession?.(session);
    else window.dispatchEvent(new Event("aij:focus-handover"));
  }, [desktop, host, session]);
  return (
    <SessionOverview
      model={model}
      visible={visible}
      onGoTo={desktop && !host.focusSession ? undefined : goTo}
      onPick={(recipeId, label) => onHandover(session.id, recipeId, label)}
      switchBack={switchBackSuggestions.find((suggestion) => suggestion.session_id === session.id) ?? null}
      onSwitchBack={onSwitchBack ? () => onSwitchBack(session.id) : undefined}
    />
  );
}
