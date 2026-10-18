import { useState } from "react";
import { Button, Card, Chip } from "../components/primitives";
import { IconJuggler, IconProfiles } from "../components/icons";
import { signedIn, type AccountInfo } from "../lib/bridge";
import { LoginQueueCard } from "../components/LoginQueueCard";

/**
 * The get-ready home: any account that isn't signed in surfaces here as a
 * one-click fix (the vendor's own flow in an embedded terminal) — never a
 * silent dead terminal. This is the default landing until every account
 * is ready (or the user explicitly proceeds).
 */
export function ReadyHome({
  accounts,
  onSkip,
  openTerminal,
}: {
  accounts: AccountInfo[];
  onSkip: () => void;
  /** AJ-E308: the shared queue card spawns its own terminals. */
  openTerminal: (id: string, title: string) => void;
}) {
  const ready = accounts.filter((a) => signedIn(a.login_state)).length;
  // AJ-E308 Req.3: the expanded shared queue card per account.
  const [openFor, setOpenFor] = useState<string | null>(null);
  const total = accounts.length;
  const allReady = ready === total;

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-5">
      <header className="flex flex-col items-center gap-2 pt-6 text-center">
        <span className="text-accent drop-shadow-[0_0_14px_#7c7cf455]">
          <IconJuggler width={30} height={30} />
        </span>
        <h2 className="text-lg font-semibold tracking-tight">
          {allReady ? "All accounts ready" : "Get your accounts ready"}
        </h2>
        <p className="max-w-md text-xs leading-relaxed text-muted">
          Each account needs Claude's own sign-in once — it opens right here
          in an embedded terminal and takes about thirty seconds. AI Juggler
          never sees your credentials.
        </p>
        <Chip tone={allReady ? "success" : "accent"}>
          {ready} of {total} ready
        </Chip>
      </header>

      <div className="flex flex-col gap-2">
        {accounts.map((account) => {
          const isReady = signedIn(account.login_state);
          return (
            <Card
              key={account.id}
              className={
                "flex items-center gap-3 p-3.5 " +
                (isReady ? "border-success/25" : "")
              }
            >
              <span
                className={
                  "size-2 shrink-0 rounded-full " +
                  (isReady ? "bg-success" : "bg-live animate-breathe")
                }
              />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{account.label}</p>
                <p className="text-[11px] text-faint">
                  {isReady
                    ? "signed in — launch anytime"
                    : "needs Claude sign-in (once)"}
                </p>
              </div>
              {account.plan && <Chip tone="neutral">{account.plan}</Chip>}
              {isReady ? (
                <Chip tone="success">ready</Chip>
              ) : (
                <Button
                  size="sm"
                  variant="primary"
                  onClick={() => setOpenFor((current) => (current === account.id ? null : account.id))}
                  aria-expanded={openFor === account.id}
                >
                  <IconProfiles width={13} height={13} />
                  Sign in
                </Button>
              )}
            </Card>
          );
        })}
        {accounts
          .filter((account) => account.id === openFor && !signedIn(account.login_state))
          .map((account) => (
            <LoginQueueCard
              key={account.id}
              accountId={account.id}
              label={account.label}
              providerName="claude-code"
              initialState={account.login_state}
              onSpawned={(terminalId) =>
                openTerminal(terminalId, `Sign in · ${account.label}`)
              }
            />
          ))}
      </div>

      <div className="flex items-center justify-center gap-4">
        <button
          type="button"
          onClick={onSkip}
          className="text-xs text-muted underline-offset-2 hover:text-ink hover:underline"
        >
          {allReady ? "Continue to Projects →" : "Skip for now — show projects"}
        </button>
      </div>
    </div>
  );
}
