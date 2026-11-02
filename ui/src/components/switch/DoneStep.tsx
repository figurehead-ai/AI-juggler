/**
 * The switch's done step: where it went (0.3.5: "Opened in Claude app ·
 * Personal" on Desktop, or a terminal on the CLI) and what happens to the
 * original, what was set up, and (0.3) what the other tool was told: a
 * switch that wrote a handover (to another tool) has a line and Open the
 * handover; one on the same tool copied the whole session and shows
 * nothing of the kind.
 */
import type { JSX } from "react";
import type { PreflightInfoData } from "../../lib/bridge";
import type { AppliedInfo } from "../../lib/ipc/switchSetup";
import { toolSpec } from "../../lib/tools";
import { outsideStays } from "../SwitchFreezeChoice";
import { SwitchSetupResults } from "../SwitchSetupList";
import { SwitchHandover } from "../HandoverView";
import { AppIcon } from "../AppIcon";
import { IconTerminal } from "../icons";
import { destinationsOf, type Destination } from "./WhereChoice";

export function DoneStep({
  preflight,
  target,
  edgeId,
  results,
  continuedIn = "cli",
}: {
  preflight: PreflightInfoData;
  target: string;
  edgeId: string;
  results: AppliedInfo[];
  continuedIn?: Destination;
}): JSX.Element {
  const desktop = continuedIn === "desktop" ? destinationsOf(preflight).desktop : null;
  const spec = toolSpec(preflight.target_tool);
  const where = desktop
    ? `Opened in ${desktop.name}`
    : continuedIn === "desktop"
      ? `Opened in the ${spec.desktop_app ?? "desktop"} app`
      : `On the CLI: ${spec.display_name} in a terminal`;
  return (
    <div className="flex flex-col gap-4" data-testid="switch-done">
      <h3 className="text-title text-ink">Switched to {target}</h3>
      <p
        className="inline-flex w-fit items-center gap-2 rounded-full border border-success/30 bg-success-soft px-3 py-1 text-body font-medium text-ink"
        data-testid="switch-done-where"
        data-where={continuedIn}
      >
        {continuedIn === "desktop" ? (
          <AppIcon appId={desktop?.app_id ?? spec.host_app ?? null} name={spec.desktop_app ?? "Desktop"} size={16} />
        ) : (
          <IconTerminal aria-hidden="true" width={16} height={16} className="shrink-0 text-success" />
        )}
        {where}
      </p>
      <p className="text-body text-ink-2" data-testid="switch-done-words">
        {preflight.source_outside
          ? outsideStays(preflight.source_app, preflight.source_live)
          : preflight.source_keeps_running
            ? `${target} is picking up the conversation. The original keeps going in its app.`
            : `${target} is picking up the conversation. The original has stopped, and you can switch back to it any time.`}
      </p>
      <SwitchHandover edgeId={edgeId} />
      <SwitchSetupResults target={target} results={results} />
    </div>
  );
}
