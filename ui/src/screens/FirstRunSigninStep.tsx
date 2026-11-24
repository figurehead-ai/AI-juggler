/**
 * First run's "Sign in where needed" step: one row per app that still
 * needs its own tool's sign-in, one after another or each on its own,
 * and how each sign-in ended (AJ-E1103). Moved out of FirstRun.tsx (its
 * size budget).
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { createPortal } from "react-dom";
import { Button } from "../components/primitives";
import { errorText } from "../lib/bridge";
import { statsEvent } from "../lib/ipc/usage-stats";
import { terminalSpawnLogin } from "../lib/ipc/terminal-io";
import { stepWords } from "./FirstRunSteps";
import { SigninRow } from "./FirstRunSigninRow";
import type { Signin } from "./FirstRun";

/** AJ-E1103 `onboarding.login`: how a first-run sign-in ended (the
 * backend maps the tool to the schema's provider names, "other" for any
 * it doesn't list). */
export function countLogin(provider: string, outcome: "completed" | "skipped" | "expired"): void {
  statsEvent("onboarding.login", { provider, outcome });
}

export function SigninStep({
  queue,
  provider,
  openLoginTerminal,
  onDone,
  wizard = false,
  footEnd,
  skipRef,
}: {
  /** In the setup wizard: its step look (0.5.1 critique). */
  wizard?: boolean;
  /** The setup wizard's foot's end: the step's primary goes there, last
   * (0.5.1 F50). */
  footEnd?: HTMLElement | null;
  /** The setup wizard's Skip this step, in place of the step's own Sign
   * in later (one way to skip, worded by what it does: 0.5.1 F50). */
  skipRef?: { current: (() => void) | null };
  provider: string;
  queue: Signin[];
  openLoginTerminal: (id: string, title: string, options?: { select?: boolean; account?: string }) => void;
  onDone: () => void;
}): JSX.Element {
  const [terminals, setTerminals] = useState<Record<string, string>>({});
  const [ready, setReady] = useState<Record<string, boolean>>({});
  const [skippedIds, setSkippedIds] = useState<Record<string, boolean>>({});
  const [active, setActive] = useState<string | null>(null);
  const [runAll, setRunAll] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const left = queue.filter((entry) => !ready[entry.accountId]);
  // Skipped, each one left counts as skipped (AJ-E1103).
  const skipAll = () => {
    for (const entry of left) if (!skippedIds[entry.accountId]) countLogin(entry.provider || provider, "skipped");
    onDone();
  };
  useEffect(() => {
    if (!skipRef) return undefined;
    skipRef.current = skipAll;
    return () => {
      skipRef.current = null;
    };
  });
  const start = useCallback(
    (entry: Signin) => {
      setError(null);
      setActive(entry.accountId);
      void terminalSpawnLogin(entry.accountId).then(
        (terminal) => {
          setTerminals((current) => ({ ...current, [entry.accountId]: terminal.id }));
          // Known to the window too (it stays reachable after this flow
          // closes), but not shown there: two views would fight its size.
          openLoginTerminal(terminal.id, `Sign in · ${entry.label}`, { select: false, account: entry.accountId });
        },
        (cause) => setError(errorText(cause)),
      );
    },
    [openLoginTerminal],
  );
  const markReady = (entry: Signin) => {
    if (!ready[entry.accountId]) countLogin(entry.provider || provider, "completed");
    setReady((current) => ({ ...current, [entry.accountId]: true }));
    if (runAll) {
      const next = queue.find((other) => other.accountId !== entry.accountId && !ready[other.accountId]);
      if (next) start(next);
      else setRunAll(false);
    }
  };
  return (
    <div className="flex flex-col gap-6" data-testid="first-run-signin">
      <div className="flex flex-col gap-2">
        <h1 className={stepWords(wizard).heading}>
          {left.length === 0 ? "Everyone's signed in." : left.length === 1 ? "Sign in to 1 app." : `Sign in to ${left.length} apps.`}
        </h1>
        {/* AJ-E1545 R8: each opens its own tool's page; no tool named here. */}
        <p className={stepWords(wizard).lead}>
          Each opens its own sign-in page; AI Juggler never sees your password.
        </p>
      </div>
      <ul className="overflow-hidden rounded-2xl border border-hairline bg-surface">
        {queue.map((entry) => (
          <SigninRow
            onSkip={queue.length > 1 ? () => {
              setSkippedIds((current) => ({ ...current, [entry.accountId]: true }));
              countLogin(entry.provider || provider, "skipped");
            } : undefined}
            key={entry.accountId}
            entry={entry}
            lead={left.length <= 1}
            active={active === entry.accountId}
            terminal={terminals[entry.accountId] ?? null}
            onStart={() => start(entry)}
            onReady={() => markReady(entry)}
          />
        ))}
      </ul>
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
      {(() => {
        const primary =
          left.length === 0 ? (
            <Button variant="primary" onClick={onDone} data-testid="first-run-signin-continue">
              Continue
            </Button>
          ) : left.length > 1 && !runAll ? (
            <Button
              variant="primary"
              onClick={() => {
                setRunAll(true);
                start(left[0]!);
              }}
              data-testid="first-run-signin-all"
            >
              Sign in to all {left.length}, one after another
            </Button>
          ) : null;
        // In the setup wizard: its foot's Skip this step, the primary last.
        if (skipRef && footEnd) return primary && createPortal(primary, footEnd);
        return (
          <div className="flex flex-wrap items-center justify-between gap-4">
            {left.length > 0 ? (
              <button type="button" className="text-sm text-muted hover:text-ink hover:underline" onClick={skipAll} data-testid="first-run-signin-next">
                Sign in later
              </button>
            ) : (
              <span />
            )}
            {primary}
          </div>
        );
      })()}
    </div>
  );
}
