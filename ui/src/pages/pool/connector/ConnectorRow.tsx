/**
 * Round 7 (AJ-E3011, Casey's worst moment): New pool's Who's in asks for
 * the connector on each desktop App's own row, before Next, not in a card
 * beside the list that was easy to pass by. The row says where the App
 * stands, "Watched: can't talk in the pool yet", with Add; Add opens the
 * consent card's own preview (the same files, the same "Add to …"), so
 * it's one component on both paths (the pool's inspector shows the full
 * card). Review keeps its warning as the backstop.
 */
import type { JSX, ReactNode } from "react";

import { Button } from "../../../components/primitives";
import { CONNECTOR, CONNECTOR_MEANING, statusLine } from "../../../lib/connector";
import type { ConnectorStatusInfo } from "../../../lib/ipc/connector";
import type { AgentBridgeInfo } from "../../../lib/ipc/agent-bridge";

/** Where a desktop App's connector stands, in its row. */
export function ConnectorRowState({
  status,
  bridge,
  testId,
  busy,
  failed,
  onReview,
  onGuide,
}: {
  status: ConnectorStatusInfo;
  bridge: AgentBridgeInfo | null;
  testId: string;
  busy: boolean;
  failed: ReactNode;
  onReview: () => void;
  onGuide: () => void;
}): JSX.Element {
  if (status.state === "current") {
    return (
      <p className="text-meta text-ink" data-testid={testId} data-state={status.state}>
        <span className="font-medium text-success">Can talk in the pool:</span> AI Juggler's connector is in it.
      </p>
    );
  }
  const can = status.state === "absent" || status.state === "outdated";
  // Why it can't talk yet: never just "Watched".
  const why = status.state === "absent" ? `AI Juggler's connector (${CONNECTOR_MEANING}) isn't in it.` : statusLine(status, bridge);
  return (
    <div className="flex flex-col gap-1.5" data-testid={testId} data-state={status.state}>
      <p className="text-meta text-ink">
        <span className="font-medium text-warning">Watched: can't talk in the pool yet.</span> <span data-testid={`${testId}-why`}>{why}</span>
      </p>
      {can && (
        <span className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant="accent" disabled={busy} onClick={onReview} data-testid={`${testId}-add`}>
            {status.state === "outdated" ? CONNECTOR.update : CONNECTOR.add}
          </Button>
          <Button size="sm" variant="ghost" disabled={busy} onClick={onGuide} data-testid={`${testId}-guide-open`}>
            {CONNECTOR.showGuide}
          </Button>
        </span>
      )}
      {failed}
    </div>
  );
}
