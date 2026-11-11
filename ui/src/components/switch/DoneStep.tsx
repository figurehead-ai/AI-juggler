/**
 * The switch's done step: where it went (0.3.5: "Opened in Claude app ·
 * Personal" on Desktop, or a terminal on the CLI) and what happens to the
 * original, what was set up, and (0.3) what the other tool was told: a
 * switch that wrote a handover (to another tool) has a line and Open the
 * handover; one on the same tool copied the whole session and shows
 * nothing of the kind.
 */
import type { JSX } from "react";
import type { PreflightInfoData } from "../../lib/ipc/switch-preflight";
import type { AppliedInfo } from "../../lib/ipc/switchSetup";
import { toolSpec } from "../../lib/tools";
import { fateOf, originalFate, outsideStays } from "../SwitchFreezeChoice";
import { SwitchSetupResults } from "../SwitchSetupList";
import { SwitchHandover } from "../HandoverView";
import { AppIcon } from "../AppIcon";
import { IconTerminal } from "../icons";
import { destinationsOf, type Destination } from "./WhereChoice";
import type { SwitchExecutedData } from "../../lib/ipc/switch-preflight";
import { restartPlace } from "../../lib/go-to";
import { Button } from "../primitives";
import { AutomationAfter } from "./AutomationLine";

/** ADR-0028: the sessions started from it that moved with it, and where
 * they nest: in the app now, or once it restarts (its records load then). */
export interface FamilyMoved {
  moved: number;
  nests: "now" | "restart" | "";
  place: string | null;
}

export function familyOf(result: SwitchExecutedData): FamilyMoved | undefined {
  const moved = result.family_moved ?? 0;
  if (moved === 0) return undefined;
  const nests = result.family_nests === "now" || result.family_nests === "restart" ? result.family_nests : "";
  return { moved, nests, place: result.family_place ?? null };
}

function familyWords(family: FamilyMoved, app: string): string {
  const sessions = family.moved === 1 ? "1 session started from it" : `${family.moved} sessions started from it`;
  const one = family.moved === 1;
  if (family.nests === "now") return `${sessions} moved with it and ${one ? "sits" : "sit"} under it in ${app}.`;
  if (family.nests === "restart") return `${sessions} moved with it. ${one ? "It sits" : "They sit"} under it in ${app} once ${app} restarts: it reads them only when it starts.`;
  return `${sessions} moved with it. Continue any of them from AI Juggler.`;
}

export function DoneStep({
  preflight,
  target,
  edgeId,
  results,
  continuedIn = "cli",
  family,
  executed,
}: {
  preflight: PreflightInfoData;
  target: string;
  edgeId: string;
  results: AppliedInfo[];
  continuedIn?: Destination;
  family?: FamilyMoved;
  /** ADR-0028: the switch's result, for what wakes it by itself. */
  executed?: SwitchExecutedData;
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
          : `${target} is picking up the conversation. ${originalFate("after", fateOf(preflight))}`}
      </p>
      {family && (
        <div className="flex flex-col items-start gap-2" data-testid="switch-done-family" data-nests={family.nests}>
          <p className="text-body text-ink-2">{familyWords(family, desktop?.name ?? spec.desktop_app ?? "the app")}</p>
          {family.nests === "restart" && family.place && (
            <Button size="sm" variant="secondary" data-testid="switch-done-family-restart" onClick={() => restartPlace(family.place!, true)}>
              Restart {desktop?.name ?? spec.desktop_app ?? "the app"}
            </Button>
          )}
        </div>
      )}
      {executed && (
        <AutomationAfter
          result={executed}
          app={desktop?.name ?? spec.desktop_app ?? "the app"}
          sourceApp={preflight.source_app ?? `the app on ${preflight.source_account ?? "the original account"}`}
        />
      )}
      <SwitchHandover edgeId={edgeId} />
      <SwitchSetupResults target={target} results={results} />
    </div>
  );
}
