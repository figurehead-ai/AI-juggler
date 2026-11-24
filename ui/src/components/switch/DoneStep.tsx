/**
 * The switch's done step: where it went (0.3.5: "Opened in Claude app ·
 * Personal" on Desktop, or a terminal on the CLI) and what happens to the
 * original, what was set up, and (0.3) what the other tool was told: a
 * switch that wrote a handover (to another tool) has a line and Open the
 * handover; one on the same tool copied the whole session and shows
 * nothing of the kind.
 */
import { useState, type JSX } from "react";
import { placesOpen } from "../../lib/ipc/places";
import { errorText } from "../../lib/bridge";
import type { PreflightInfoData } from "../../lib/ipc/switch-preflight";
import type { AppliedInfo } from "../../lib/ipc/switchSetup";
import { toolSpec } from "../../lib/tools";
import { fateOf, originalFate, outsideStays } from "../SwitchFreezeChoice";
import { SwitchSetupResults } from "../SwitchSetupList";
import { SwitchHandover } from "../HandoverView";
import { IconSubtasks, IconTerminal } from "../icons";
import { IconComputer } from "../icons-actions";
import { IconHistory } from "../icons-places";
import { ToolLogo } from "../ToolLogo";
import { Fact, FactPanel } from "./Facts";
import { DoneHead } from "./StepParts";
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

/** The conversation's own id in a Desktop key (`conversation:<account>:<id>`). */
function conversationOf(key: string | null): string | null {
  if (!key) return null;
  const at = key.lastIndexOf(":");
  return at < 0 ? null : key.slice(at + 1) || null;
}

/** 0.5.1 F69: sent but not seen open: said plainly, with Open in that App
 * (the app started first when it isn't running), and what Open did. */
function OpenThere({ app, placeId, conversation }: { app: string; placeId: string | null; conversation: string | null }): JSX.Element {
  const [said, setSaid] = useState<string | null>(null);
  const open = () =>
    placeId &&
    void placesOpen(placeId, conversation).then(
      (outcome) => setSaid(outcome.outcome),
      (cause) => setSaid(errorText(cause)),
    );
  return (
    <>
      <span className="text-meta text-ink-2" data-testid="switch-done-not-seen">
        AI Juggler couldn’t see it open there yet. Open it to pick up the work.
      </span>
      {placeId && (
        <span className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant="secondary" type="button" onClick={open} data-testid="switch-done-open">
            Open in {app}
          </Button>
          {said && (
            <output className="text-label text-muted" data-testid="switch-done-open-said">
              {said}
            </output>
          )}
        </span>
      )}
    </>
  );
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
  const app = desktop?.name ?? `the ${spec.desktop_app ?? "desktop"} app`;
  // 0.5.1 F69 (AJ-E2322's rule, as Move): "Opened" only when the app
  // showed the conversation open; else it was sent, said plainly, with
  // Open in that App.
  const seen = executed?.opened_seen === true;
  const where = continuedIn !== "desktop" ? `On the CLI: ${spec.display_name} in a terminal` : seen ? `Opened in ${app}` : `Sent to ${app}`;
  const openPlace = continuedIn === "desktop" && !seen ? (executed?.open_place_id ?? desktop?.place_id ?? null) : null;
  return (
    <div className="flex flex-col gap-6" data-testid="switch-done">
      <DoneHead title={<>Switched to {target}</>} help="Send a message there to pick up the work." />
      <FactPanel label="What happened">
        <Fact icon={continuedIn === "desktop" ? IconComputer : IconTerminal} label="Where" tone={continuedIn === "desktop" && !seen ? "plain" : "done"}>
          <span className="flex flex-col items-start gap-2">
            <span className="inline-flex min-w-0 items-center gap-2" data-testid="switch-done-where" data-where={continuedIn} data-seen={continuedIn === "desktop" ? String(seen) : undefined}>
              <ToolLogo tool={preflight.target_tool} kind={continuedIn === "desktop" ? "desktop" : "cli"} appId={desktop?.app_id ?? undefined} size={24} />
              {where}
            </span>
            {continuedIn === "desktop" && !seen && <OpenThere app={app} placeId={openPlace} conversation={conversationOf(executed?.conversation_key ?? null)} />}
          </span>
        </Fact>
        <Fact icon={IconHistory} label="Original">
          <span data-testid="switch-done-words">
            {preflight.source_outside
              ? outsideStays(preflight.source_app, preflight.source_live)
              : `${target} is picking up the conversation. ${originalFate("after", fateOf(preflight))}`}
          </span>
        </Fact>
        {family && (
          <Fact icon={IconSubtasks} label="Its sessions">
            <span className="flex flex-col items-start gap-2" data-testid="switch-done-family" data-nests={family.nests}>
              <span>{familyWords(family, desktop?.name ?? spec.desktop_app ?? "the app")}</span>
              {family.nests === "restart" && family.place && (
                <Button size="sm" variant="secondary" data-testid="switch-done-family-restart" onClick={() => restartPlace(family.place!, true)}>
                  Restart {desktop?.name ?? spec.desktop_app ?? "the app"}
                </Button>
              )}
            </span>
          </Fact>
        )}
        {executed && (
          <AutomationAfter
            result={executed}
            app={desktop?.name ?? spec.desktop_app ?? "the app"}
            sourceApp={preflight.source_app ?? `the app on ${preflight.source_account ?? "the original account"}`}
          />
        )}
      </FactPanel>
      <SwitchHandover edgeId={edgeId} />
      <SwitchSetupResults target={target} results={results} />
    </div>
  );
}
