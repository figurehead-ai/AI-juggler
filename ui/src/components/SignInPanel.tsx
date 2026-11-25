/**
 * AJ-E1553 / AJ-E1545 R8 — a sign-in's own page (the rail's Sign-ins open
 * it): its App's tool icon, its name and a Terminal chip, then the sign-in
 * card (the tool's page and code, its outcome, or its terminal). Nothing
 * here asks the user to check again from Accounts: AI Juggler sees the
 * sign-in finish and says so.
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { SignInCard } from "./SignInCard";
import { ToolLogo } from "./ToolLogo";
import { errorText } from "../lib/bridge";
import { signInsDismiss } from "../lib/ipc/sign-ins";
import { terminalKill, terminalSpawnLogin } from "../lib/ipc/terminal-io";
import { useSignIns } from "../lib/sign-ins-store";
import type { AccountInfo } from "../types/generated/AccountInfo";

/** The chip every sign-in row carries: it runs in a terminal. */
export function TerminalChip(): JSX.Element {
  return (
    <span className="shrink-0 rounded-md bg-surface-3 px-1.5 text-label text-ink-2" data-testid="sign-in-terminal-chip">
      Terminal
    </span>
  );
}

export function SignInPanel({
  terminal,
  accounts,
  openLoginTerminal,
  onClosed,
}: {
  terminal: { id: string; title: string; ended: boolean; account?: string };
  accounts: AccountInfo[];
  /** Try again's new sign-in joins the window and is shown. */
  openLoginTerminal?: (id: string, title: string, options?: { account?: string }) => void;
  /** The sign-in left the list (Done, Dismiss). */
  onClosed?: () => void;
}): JSX.Element {
  const signIns = useSignIns();
  const signIn = signIns.find((one) => one.id === terminal.id) ?? null;
  const account = accounts.find((row) => row.id === (signIn?.account_id ?? terminal.account));
  const tool = signIn?.tool ?? account?.provider ?? "";
  const [error, setError] = useState<string | null>(null);
  const running = (signIn?.state ?? (terminal.ended ? "ended" : "running")) === "running";

  const dismiss = async () => {
    setError(null);
    await signInsDismiss(terminal.id).catch(() => null);
    onClosed?.();
  };
  const tryAgain = async () => {
    const accountId = signIn?.account_id ?? terminal.account;
    if (!accountId) return;
    setError(null);
    try {
      const next = await terminalSpawnLogin(accountId);
      await signInsDismiss(terminal.id).catch(() => null);
      openLoginTerminal?.(next.id, terminal.title, { account: accountId });
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 overflow-y-auto" data-testid="sign-in-panel">
      <header className="flex flex-wrap items-center gap-3">
        {tool && <ToolLogo tool={tool} kind="cli" size={32} />}
        <h2 className="text-lg font-semibold tracking-tight">{terminal.title}</h2>
        <TerminalChip />
        {running && (
          <div className="ml-auto">
            <Button size="sm" variant="secondary" onClick={() => void terminalKill(terminal.id)}>
              Stop
            </Button>
          </div>
        )}
      </header>
      <div className="max-w-3xl rounded-2xl bg-surface p-5">
        <SignInCard
          signIn={signIn}
          terminalId={terminal.id}
          tool={tool}
          title={terminal.title}
          tall
          onTryAgain={() => void tryAgain()}
          onDismiss={() => void dismiss()}
        />
      </div>
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
