/**
 * AJ-E3013 (D-116): the consent card for each desktop App among some
 * members (an App, or a session another app runs), once per App — New
 * pool's Who's in shows them as soon as one is in, the same card the
 * pool's inspector shows.
 */
import type { JSX } from "react";

import { useApps } from "../../../lib/apps";
import { desktopAppsOf } from "../../../lib/connector";
import { ConnectorCard } from "./ConnectorCard";

export function DesktopConnectors({ refs, where, testId = "desktop-connectors" }: { refs: readonly string[]; where?: string; testId?: string }): JSX.Element | null {
  const apps = useApps();
  const desktop = desktopAppsOf(refs, apps ?? []);
  if (desktop.length === 0) return null;
  return (
    <div className="flex flex-col gap-2" data-testid={testId}>
      {desktop.map((app) => (
        <ConnectorCard key={app.id} appId={app.id} app={app.name} where={where} heading={4} testId={`${testId}-card`} />
      ))}
    </div>
  );
}
