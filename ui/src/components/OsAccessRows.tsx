/**
 * AJ-E1527 — the optional permissions' rows, one component for first
 * run's access step, the welcome's "Make it smoother" card and Home's
 * reminder: each permission's state and its Allow, Open System Settings
 * or Reopen action (`lib/osAccess`'s cards, #120's Reopen), re-checked
 * when the window comes back.
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";
import { type AccessAction, accessCard, reopenApp, watchAccess } from "../lib/osAccess";
import { Button } from "./primitives";
import { type OsAccessInfo, type ReopenBlockInfo, errorText, osAccessList, osAccessOpenSettings, osAccessRequest } from "../lib/bridge";
import { Check } from "./StepCheck";

export function OsAccessRows({
  entries,
  setEntries,
  size = "large",
  testIdPrefix = "first-run-access",
  aside,
}: {
  entries: OsAccessInfo[];
  setEntries: (next: OsAccessInfo[]) => void;
  /** First run's big cards, or the welcome's and Home's compact rows. */
  size?: "large" | "compact";
  testIdPrefix?: string;
  /** What else a row offers beside its action (Home's "Not now"). */
  aside?: (entry: OsAccessInfo) => ReactNode;
}): JSX.Element {
  const [error, setError] = useState<string | null>(null);
  // Why a Reopen has to be confirmed or wait, under the row that asked.
  const [reopenBlock, setReopenBlock] = useState<ReopenBlockInfo | null>(null);
  useEffect(() => watchAccess(() => void osAccessList().then(setEntries, () => null)), [setEntries]);
  const act = (entry: OsAccessInfo, action: AccessAction, anyway = false) => {
    setError(null);
    if (action === "reopen") {
      void reopenApp(anyway).then(setReopenBlock, (cause) => setError(errorText(cause)));
      return;
    }
    const step = action === "settings" ? osAccessOpenSettings(entry.id).then(() => osAccessList()) : osAccessRequest(entry.id);
    void step.then(setEntries, (cause) => setError(errorText(cause)));
  };
  const large = size === "large";
  const buttonSize = large ? undefined : "sm";
  return (
    <>
      <ul className={"flex flex-col " + (large ? "gap-3" : "gap-2")}>
        {entries.map((entry) => {
          const card = accessCard(entry);
          return (
            <li
              key={entry.id}
              className={
                "flex flex-wrap items-center gap-4 border " +
                (large ? "rounded-2xl bg-surface p-5 " : "rounded-xl bg-surface-2 px-4 py-3 ") +
                (card.state === "allowed" ? "border-success/40" : "border-hairline")
              }
              data-testid={`${testIdPrefix}-${entry.id}`}
            >
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <span className={(large ? "text-[15px]" : "text-sm") + " font-semibold text-ink"}>{entry.title}</span>
                <span className={(large ? "text-sm" : "text-[13px]") + " leading-relaxed text-muted"}>
                  {entry.why} <span className="text-faint">The system calls this {entry.system_name}.</span>
                </span>
                {card.note && <span className={card.noteLeads ? "text-sm text-ink" : "text-xs text-faint"}>{card.note}</span>}
                {card.state === "reopen" && reopenBlock && (
                  <div className="mt-2 flex flex-wrap items-center gap-3 text-sm" role="alert" data-testid={`${testIdPrefix}-reopen-block`}>
                    <span className="text-ink">{reopenBlock.reason}</span>
                    {reopenBlock.can_reopen_anyway && (
                      <Button size="sm" variant="secondary" onClick={() => act(entry, "reopen", true)}>
                        Reopen anyway
                      </Button>
                    )}
                    <button type="button" className="text-sm text-muted hover:text-ink hover:underline" onClick={() => setReopenBlock(null)}>
                      Not now
                    </button>
                  </div>
                )}
              </div>
              {card.state === "allowed" ? (
                <span className="flex items-center gap-1.5 text-sm font-medium text-success" role="status">
                  <Check />
                  Allowed
                </span>
              ) : (
                <div className="flex flex-wrap items-center gap-2">
                  {aside?.(entry)}
                  {card.secondary && (
                    <Button size={buttonSize} variant="secondary" onClick={() => act(entry, card.secondary!.action)}>
                      {card.secondary.label} <span className="sr-only">{entry.title}</span>
                    </Button>
                  )}
                  {card.primary && (
                    <Button
                      size={buttonSize}
                      variant={card.primary.action === "settings" ? "secondary" : "primary"}
                      onClick={() => act(entry, card.primary!.action)}
                      data-testid={card.primary.action === "reopen" ? `${testIdPrefix}-reopen` : undefined}
                    >
                      {card.primary.label}
                      {card.primary.action !== "reopen" && (
                        <>
                          {" "}
                          <span className="sr-only">{entry.title}</span>
                        </>
                      )}
                    </Button>
                  )}
                </div>
              )}
            </li>
          );
        })}
      </ul>
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </>
  );
}
