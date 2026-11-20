/**
 * D-126: New pool's Who's in asks for AI Juggler's connector in one line
 * under a chosen desktop App's card, as mocked: "Claude needs AI
 * Juggler's connector to message the team. You see the exact change
 * before anything is written." and Preview and add. That one click opens
 * the consent card's own preview (`startWith="preview"`: the button was
 * its Add), so the files, "Add to …" and what Add did are one component
 * on both paths; Cancel brings the line back. Where the App stands
 * otherwise (the connector in, can't take it, changed) is the card's own
 * line.
 */
import { useState, type JSX } from "react";

import { Button } from "../../../components/primitives";
import { IconConnect } from "../../../components/icons-menu";
import { CONNECTOR, useConnectorStatus } from "../../../lib/connector";
import { ConnectorCard } from "./ConnectorCard";

export function ConnectorNotice({ appId, app, kind, profile, testId }: { appId: string; /** The App as named ("Claude · Work"). */ app: string; kind?: string; profile?: string | null; testId: string }): JSX.Element {
  const status = useConnectorStatus(appId);
  const [open, setOpen] = useState(false);
  const state = status?.state;
  const asks = state === "absent" || state === "outdated";
  const card = { appId, app, kind, profile, heading: 4 as const, testId: `${testId}-card` };
  return (
    <div className={"col-span-full flex gap-3 rounded-xl bg-surface-2 px-3.5 py-3 " + (asks && !open ? "items-center" : "items-start")} data-testid={testId} data-state={state}>
      <IconConnect aria-hidden="true" width={16} height={16} className={"shrink-0 text-faint" + (asks && !open ? "" : " mt-0.5")} />
      {open ? (
        <div className="min-w-0 flex-1">
          <ConnectorCard {...card} startWith="preview" onCancel={() => setOpen(false)} />
        </div>
      ) : asks ? (
        <>
          <p className="min-w-0 flex-1 text-meta text-ink-2" data-testid={`${testId}-line`}>
            <span className="font-medium text-ink">{state === "outdated" ? CONNECTOR.older(app) : CONNECTOR.needs(app)}</span> {CONNECTOR.seeFirst}
          </p>
          <Button size="sm" variant="accent" className="shrink-0" onClick={() => setOpen(true)} data-testid={`${testId}-preview`}>
            {state === "outdated" ? CONNECTOR.previewAndUpdate : CONNECTOR.previewAndAdd}
          </Button>
        </>
      ) : (
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <p className="text-meta font-medium text-ink">{app}</p>
          <ConnectorCard {...card} compact />
        </div>
      )}
    </div>
  );
}
