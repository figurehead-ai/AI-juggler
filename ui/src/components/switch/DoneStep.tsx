/**
 * The switch's done step: where it went and what happens to the
 * original, what was set up, and (0.3) what the other tool was told: a
 * switch that wrote a handover (to another tool) has a line and Open the
 * handover; one on the same tool copied the whole session and shows
 * nothing of the kind.
 */
import type { JSX } from "react";
import type { PreflightInfoData } from "../../lib/bridge";
import type { AppliedInfo } from "../../lib/ipc/switchSetup";
import { outsideWords } from "../SwitchFreezeChoice";
import { SwitchSetupResults } from "../SwitchSetupList";
import { SwitchHandover } from "../HandoverView";

export function DoneStep({ preflight, target, edgeId, results }: { preflight: PreflightInfoData; target: string; edgeId: string; results: AppliedInfo[] }): JSX.Element {
  return (
    <div className="flex flex-col gap-4" data-testid="switch-done">
      <h3 className="text-title text-ink">Switched to {target}</h3>
      <p className="text-body text-ink" data-testid="switch-done-words">
        {preflight.source_outside
          ? outsideWords(preflight.source_app, preflight.source_live, target)
          : preflight.source_keeps_running
            ? `${target} is picking up the conversation. The original keeps going in its app.`
            : `${target} is picking up the conversation. The original has stopped, and you can switch back to it any time.`}
      </p>
      <SwitchHandover edgeId={edgeId} />
      <SwitchSetupResults target={target} results={results} />
    </div>
  );
}
