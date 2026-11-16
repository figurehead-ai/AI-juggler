/**
 * AJ-E3013 (D-116): when Add isn't offered, or the preview refuses, the
 * step-by-step guide to add AI Juggler's connector yourself — the shared
 * Show me how (the switch's own, with its command and Copy) — and its
 * completion seen: while the steps are shown, and only then, AI Juggler
 * reads the App's connector, and in a pool its desktop members, every few
 * seconds; done when the connector is there or one of the App's sessions
 * calls. When there's no way to do it yourself either, it says so.
 */
import { useState, type JSX } from "react";

import { TurnOnSteps } from "../../../components/SwitchSetupTurnOn";
import { isAdded, useConnectorStatus, useDesktopMembers } from "../../../lib/connector";
import type { GetItInfo } from "../../../lib/ipc/switchSetup";

export function ConnectorGuide({
  appId,
  app,
  guide,
  poolId,
  why,
  testId,
}: {
  appId: string;
  /** The App, as named ("Claude · Work"). */
  app: string;
  /** The steps (the preview's), or `null`: there are none for this app. */
  guide: GetItInfo | null;
  /** In a pool, a session's first call through it counts as done too. */
  poolId?: string;
  /** Why Add can't do it here, said first. */
  why?: string | null;
  testId: string;
}): JSX.Element {
  const [open, setOpen] = useState(true);
  const status = useConnectorStatus(appId, open && guide !== null);
  const members = useDesktopMembers(poolId, open && guide !== null && !!poolId);
  const connected = [...(members?.values() ?? [])].some((one) => one.app_id === appId && one.connected);
  const done = isAdded(status?.state) || connected;
  return (
    <div className="flex flex-col gap-1.5" data-testid={`${testId}-guide`}>
      {why && <p className="text-meta text-ink-2">{why}</p>}
      {guide === null ? (
        <p className="text-meta text-ink" data-testid={`${testId}-guide-none`}>
          There's no way to add it to {app} yourself yet either, so its sessions stay Watched in pools.
        </p>
      ) : done ? (
        <output className="block text-meta font-medium text-ink" data-testid={`${testId}-guide-done`}>
          {connected ? `Done: a session of ${app} called AI Juggler, so it's connected.` : `Done: AI Juggler's connector is in ${app}.`}
        </output>
      ) : (
        <>
          <TurnOnSteps getIt={guide} name="AI Juggler's connector" startOpen onOpen={setOpen} />
          {open && (
            <output className="block text-meta text-muted" data-testid={`${testId}-guide-watching`}>
              Watching for it while these steps are open.
            </output>
          )}
        </>
      )}
    </div>
  );
}
