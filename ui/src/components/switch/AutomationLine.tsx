/**
 * ADR-0028 (`automation_handover`): what wakes the conversation by itself
 * (Auto-fix on a pull request, a Codex goal), named before the switch and
 * after it, so two copies never work at once and nobody archives by hand.
 *
 * Before: one line saying the copy takes it over and the original's is
 * turned off; when the switch continues in a terminal and the tool keeps
 * it in its desktop app, it stays with the original instead, said in
 * amber. What belongs to the account (a Codex heartbeat) is said as
 * staying.
 *
 * After: what the copy took over, and when each side takes effect; the
 * app the copy runs in reads it at its next start, so Restart is offered.
 */
import type { JSX } from "react";
import type { PreflightInfoData, SwitchExecutedData } from "../../lib/ipc/switch-preflight";
import { restartPlace } from "../../lib/go-to";
import { Button } from "../primitives";
import { IconClock } from "../icons-menu";
import { Fact } from "./Facts";
import type { Destination } from "./WhereChoice";

/** "A", "A and B", "A, B and C". */
export function joined(items: string[]): string {
  if (items.length <= 1) return items[0] ?? "";
  return `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

export function AutomationBefore({ preflight, destination, app }: { preflight: PreflightInfoData; destination: Destination; app: string }): JSX.Element | null {
  const moves = preflight.automation ?? [];
  const stays = preflight.automation_stays ?? [];
  if (moves.length === 0 && stays.length === 0) return null;
  const left = moves.length > 0 && preflight.automation_needs_desktop === true && destination === "cli";
  return (
    <Fact icon={IconClock} label="Runs by itself" tone={left ? "warn" : "plain"}>
      <span className="flex flex-col gap-1" data-testid="switch-automation">
        {moves.length > 0 &&
          (left ? (
            <span className="font-medium" data-testid="switch-automation-left">
              {joined(moves)} {moves.length === 1 ? "stays" : "stay"} with the original: only {app} runs {moves.length === 1 ? "it" : "them"}. Continue in {app} to take {moves.length === 1 ? "it" : "them"} along.
            </span>
          ) : (
            <span data-testid="switch-automation-moves">
              {joined(moves)} {moves.length === 1 ? "moves" : "move"} to the copy and {moves.length === 1 ? "is" : "are"} turned off on the original, so only one of them works.
            </span>
          ))}
        {stays.map((line) => (
          <span key={line} data-testid="switch-automation-stays">
            {line}.
          </span>
        ))}
      </span>
    </Fact>
  );
}

export function AutomationAfter({ result, app, sourceApp }: { result: SwitchExecutedData; app: string; sourceApp: string }): JSX.Element | null {
  const moved = result.automation_moved ?? [];
  const kept = result.automation_kept ?? [];
  if (moved.length === 0 && kept.length === 0) return null;
  const place = result.automation_copy_place ?? null;
  const one = moved.length === 1;
  return (
    <Fact icon={IconClock} label="Runs by itself">
      <span className="flex flex-col items-start gap-2" data-testid="switch-done-automation">
        {moved.length > 0 && (
          <span>
            {place
              ? `${joined(moved)} ${one ? "runs" : "run"} in the copy once ${app} restarts: it reads ${one ? "it" : "them"} only when it starts.`
              : `${joined(moved)} now ${one ? "runs" : "run"} in the copy.`}{" "}
            {result.automation_original_waits
              ? `On the original ${one ? "it's" : "they're"} turned off when ${sourceApp} next closes (it reads the change only when it starts).`
              : `On the original ${one ? "it's" : "they're"} off; switching back turns ${one ? "it" : "them"} on there again.`}
          </span>
        )}
        {place && (
          <Button size="sm" variant="secondary" data-testid="switch-done-automation-restart" onClick={() => restartPlace(place, true)}>
            Restart {app}
          </Button>
        )}
        {kept.map((line) => (
          <span key={line}>{line}.</span>
        ))}
      </span>
    </Fact>
  );
}
