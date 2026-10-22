/**
 * First run's Computer access step (AJ-E1525, #120's Reopen): wiring for
 * `lib/osAccess`. Its own module (the size budget), on the way to the
 * shared access rows (AJ-E1527).
 */
import { useEffect, useState, type JSX } from "react";
import { type AccessAction, accessCard, reopenApp, watchAccess } from "../lib/osAccess";
import { Button } from "./primitives";
import { type OsAccessInfo, type ReopenBlockInfo, errorText, osAccessList, osAccessOpenSettings, osAccessRequest } from "../lib/bridge";
import { Check } from "./StepCheck";

/** First run's Computer access step: wiring for `lib/osAccess` (the
 * cards' logic, the re-check when the window is back, Reopen). */
export function AccessStep({
  entries,
  setEntries,
  onDone,
}: {
  entries: OsAccessInfo[];
  setEntries: (next: OsAccessInfo[]) => void;
  onDone: () => void;
}): JSX.Element {
  const [error, setError] = useState<string | null>(null);
  // Why a Reopen has to be confirmed or wait, under the card that asked.
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
  const allAllowed = entries.every((entry) => entry.status === "allowed");
  return (
    <div className="flex flex-col gap-6" data-testid="first-run-access">
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">
          {allAllowed
            ? "All set."
            : entries.length === 1
              ? "One optional permission makes it smoother."
              : `${entries.length} optional permissions make it smoother.`}
        </h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          Allow opens the system's own prompt, and this page updates as soon as you say yes. Skip any of them: AI Juggler asks again only when you first use what it's for.
        </p>
      </div>
      <ul className="flex flex-col gap-3">
        {entries.map((entry) => {
          const card = accessCard(entry);
          return (
            <li
              key={entry.id}
              className={
                "flex flex-wrap items-center gap-4 rounded-2xl border bg-surface p-5 " +
                (card.state === "allowed" ? "border-success/40" : "border-hairline")
              }
              data-testid={`first-run-access-${entry.id}`}
            >
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <span className="text-[15px] font-semibold text-ink">{entry.title}</span>
                <span className="text-sm leading-relaxed text-muted">
                  {entry.why} <span className="text-faint">The system calls this {entry.system_name}.</span>
                </span>
                {card.note && <span className={card.noteLeads ? "text-sm text-ink" : "text-xs text-faint"}>{card.note}</span>}
                {card.state === "reopen" && reopenBlock && (
                  <div className="mt-2 flex flex-wrap items-center gap-3 text-sm" role="alert" data-testid="first-run-access-reopen-block">
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
                  {card.secondary && (
                    <Button variant="secondary" onClick={() => act(entry, card.secondary!.action)}>
                      {card.secondary.label} <span className="sr-only">{entry.title}</span>
                    </Button>
                  )}
                  {card.primary && (
                    <Button
                      variant={card.primary.action === "settings" ? "secondary" : "primary"}
                      onClick={() => act(entry, card.primary!.action)}
                      data-testid={card.primary.action === "reopen" ? "first-run-access-reopen" : undefined}
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
      <div className="flex items-center justify-between gap-4">
        {!allAllowed ? (
          <button type="button" className="text-sm text-muted hover:text-ink hover:underline" onClick={onDone}>
            Skip for now
          </button>
        ) : (
          <span />
        )}
        <Button variant="primary" onClick={onDone} data-testid="first-run-start">
          Start working
        </Button>
      </div>
    </div>
  );
}
