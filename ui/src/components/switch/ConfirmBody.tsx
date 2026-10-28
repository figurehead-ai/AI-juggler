/**
 * The switch's last screen (the one-click door, and the wizard's
 * Confirm step), to read (the maintainer, 2026-10-27: "very hard to
 * read … leaving the user in a hanging place"):
 *
 * - "Ready to switch to Default", and at most three lines: what moves,
 *   what happens to the original, and whether anything needs you;
 * - "Before you switch" only when something does: real risks (low quota
 *   on the target), the original's stop choice while it runs, what can be
 *   set up, and what the account turns on itself, each with its action;
 * - everything that's fine behind "Details", closed until opened (and
 *   remembered).
 */
import { useState, type JSX, type ReactNode } from "react";
import type { PreflightInfoData } from "../../lib/bridge";
import type { SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { badgeWords } from "../../lib/switch-words";
import { toolSpec } from "../../lib/tools";
import { originalWords, SwitchFreezeChoice, type FreezeChoice } from "../SwitchFreezeChoice";
import { SwitchSetupList } from "../SwitchSetupList";
import { CarryList } from "./CarryList";

const DETAILS_KEY = "aij.switch.details";

function readOpen(): boolean {
  try {
    return localStorage.getItem(DETAILS_KEY) === "open";
  } catch {
    return false;
  }
}

/** What needs the user among the setup offers: something only they can
 * do, or turn on in the account. */
export function needsYouCount(setup: SetupCheckInfo | null): number {
  return (setup?.offers ?? []).filter((offer) => offer.verdict === "needs-you" || offer.steps.length > 0).length;
}

export function ConfirmBody({
  preflight,
  target,
  freeze,
  onFreeze,
  setup,
  picked,
  onToggle,
  onRecheck,
  checking,
  extra,
}: {
  preflight: PreflightInfoData;
  target: string;
  freeze: FreezeChoice;
  onFreeze: (choice: FreezeChoice) => void;
  setup: SetupCheckInfo | null;
  picked: ReadonlySet<string>;
  onToggle: (id: string) => void;
  onRecheck: () => void;
  checking: boolean;
  /** Anything else that needs an answer first (the keys question). */
  extra?: ReactNode;
}): JSX.Element {
  const [open, setOpen] = useState(readOpen);
  const outside = preflight.source_outside ? { app: preflight.source_app ?? null, target } : null;
  const needs = needsYouCount(setup);
  const offers = setup?.offers.length ?? 0;
  const stoppable = preflight.source_live && !preflight.source_keeps_running && !outside;
  const before = preflight.warnings.length > 0 || stoppable || offers > 0 || extra !== undefined;
  const tool = toolSpec(preflight.target_tool).display_name;
  return (
    <div className="flex flex-col gap-5" data-testid="switch-step-confirm">
      <div className="flex flex-col gap-2">
        <h3 className="text-title text-ink">Ready to switch to {target}</h3>
        <p className="text-meta text-ink" data-testid="switch-cost">
          <span className="rounded-md border border-accent/40 px-1.5 py-0.5">{badgeWords(preflight.fidelity_words, preflight.validated)}</span>
        </p>
        <ul className="flex list-disc flex-col gap-1 pl-5 text-body text-ink" data-testid="switch-summary">
          <li>{preflight.move_line}</li>
          <li data-testid={outside ? "switch-outside" : preflight.source_keeps_running ? "switch-keeps-running" : "switch-original"}>
            {originalWords({ live: preflight.source_live, keepsRunning: preflight.source_keeps_running, outside, choice: freeze })}
          </li>
          <li data-testid="switch-summary-needs">
            {needs === 0
              ? setup && offers === 0
                ? `${target} has everything this account has set up. Nothing needs you.`
                : "Nothing needs you."
              : needs === 1
                ? "1 thing needs you below; it doesn't stop the switch."
                : `${needs} things need you below; none of them stops the switch.`}
          </li>
        </ul>
      </div>

      {before && (
        <section className="flex flex-col gap-4 border-t border-hairline pt-4" aria-labelledby="switch-before" data-testid="switch-before">
          <h4 id="switch-before" className="text-section text-ink">
            Before you switch
          </h4>
          {preflight.warnings.length > 0 && (
            <ul className="flex flex-col gap-1.5" data-testid="switch-warnings">
              {preflight.warnings.map((warning) => (
                <li key={warning} className="rounded-lg border border-warning/40 px-3 py-2 text-body text-warning">
                  {warning}
                </li>
              ))}
            </ul>
          )}
          {extra}
          {stoppable && <SwitchFreezeChoice sourceLive value={freeze} onChange={onFreeze} />}
          {setup && offers > 0 && <SwitchSetupList check={setup} picked={picked} onToggle={onToggle} onRecheck={onRecheck} checking={checking} />}
        </section>
      )}

      <details
        open={open}
        onToggle={(event) => {
          const now = (event.target as HTMLDetailsElement).open;
          setOpen(now);
          try {
            localStorage.setItem(DETAILS_KEY, now ? "open" : "closed");
          } catch {
            /* this visit only */
          }
        }}
        className="border-t border-hairline pt-3"
        data-testid="switch-details"
      >
        <summary className="min-h-6 cursor-pointer rounded text-body font-semibold text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60">Details</summary>
        <div className="mt-3 flex flex-col gap-3">
          <p className="text-body text-ink" data-testid="switch-oneclick-carry">
            {preflight.experimental_copy}
          </p>
          {(preflight.carry_items?.length ?? 0) > 0 && <CarryList items={preflight.carry_items} target={target} tool={tool} />}
          {setup && setup.already_there > 0 && (
            <p className="text-body text-ink">
              {target} already has {setup.already_there} of the things this account has set up.
            </p>
          )}
        </div>
      </details>
    </div>
  );
}
