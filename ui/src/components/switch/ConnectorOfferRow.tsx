/**
 * AJ-E3013 (D-116): in the switch's set-up list (and a new account's
 * settings), AI Juggler's connector is AI Juggler's own, never one of the
 * user's skills: it's never ticked or copied (a copy would leave its
 * manifest behind). Its row says so, and offers the connector's own Add
 * for the target's desktop app, on the same consent card a pool shows.
 */
import type { JSX } from "react";

import { useApps } from "../../lib/apps";
import { CONNECTOR, folderOfApp } from "../../lib/connector";
import type { OfferInfo, SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { ConnectorCard } from "../../pages/pool/connector/ConnectorCard";

export function ConnectorOfferRow({ offer, check, toFolder }: { offer: OfferInfo; check: SetupCheckInfo; toFolder?: string }): JSX.Element {
  const apps = useApps();
  const app = toFolder ? apps?.find((one) => one.kind === "desktop" && folderOfApp(one) === toFolder) : undefined;
  return (
    <li className="flex flex-col gap-1.5" data-testid="switch-setup-connector" data-offer={offer.id}>
      <span className="flex flex-wrap items-baseline gap-2">
        <span className="text-body font-medium text-ink">AI Juggler's connector</span>
        <span className="text-label text-muted">AI Juggler's own</span>
      </span>
      <span className="text-meta text-ink-2">
        {CONNECTOR.meaning} Not one of your skills, so it isn't copied. {app ? `Add it to ${app.name} with your OK, and its sessions talk in pools:` : `${check.to}'s Claude app gets it from AI Juggler, with your OK, when it joins a pool.`}
      </span>
      {app && <ConnectorCard appId={app.id} app={app.name} kind={app.app} profile={app.profile} where="pools" heading={4} testId="switch-connector" />}
    </li>
  );
}
