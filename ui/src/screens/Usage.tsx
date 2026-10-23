/**
 * D-25 — Usage: every account's runway first (what's left, when it
 * resets), then what your sessions used, by model and day. With nothing
 * set up it says what will appear, and how to get there — never a blank.
 */
import { accountColor } from "../lib/palette";
import { ToolMark } from "../components/ToolMark";
import { WelcomePointer } from "../components/Welcome";
import { useEffect, useState, type JSX } from "react";
import { AccountRunway } from "../components/AccountRunway";
import { BurnDowns } from "../components/BurnDown";
import { CostMatrix } from "../components/CostMatrix";
import { Button, Card } from "../components/primitives";
import { quotaDisplayAll, type AccountInfo, type InstanceInfo, type QuotaDisplayData } from "../lib/bridge";

export function UsageScreen({
  accounts,
  sessions,
  onAddAccounts,
  onOpenHome,
}: {
  accounts: AccountInfo[];
  sessions: InstanceInfo[];
  onAddAccounts: () => void;
  /** AJ-E1524: back to the welcome while no account is in. */
  onOpenHome?: () => void;
}): JSX.Element {
  const [displays, setDisplays] = useState<QuotaDisplayData[]>([]);
  useEffect(() => {
    let alive = true;
    const load = () => void quotaDisplayAll().then((rows) => alive && setDisplays(rows), () => null);
    load();
    const timer = window.setInterval(load, 30_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6" data-testid="usage-screen">
      <header>
        <h2 className="text-lg font-semibold tracking-tight">Usage</h2>
        <p className="text-xs text-muted">How much each account has left, and what your sessions used — measured on this Mac.</p>
      </header>

      {accounts.length === 0 ? (
        <Card className="flex flex-col items-start gap-3 p-5" data-testid="usage-no-accounts">
          <p className="text-sm font-medium">Nothing to measure yet</p>
          <p className="max-w-xl text-xs leading-relaxed text-muted">
            Once your accounts are in, this shows each one's 5-hour and weekly windows — what's left and when they reset —
            and what every session cost, by model and day.
          </p>
          <Button size="sm" variant="primary" onClick={onAddAccounts}>
            Find my accounts
          </Button>
          {onOpenHome && <WelcomePointer onOpenHome={onOpenHome} />}
        </Card>
      ) : (
        <section aria-labelledby="usage-runway" className="flex flex-col gap-2.5">
          <h3 id="usage-runway" className="text-[11px] font-semibold uppercase tracking-wider text-muted">
            Runway
          </h3>
          <ul className="grid gap-3 md:grid-cols-2">
            {accounts.map((account) => (
              <li key={account.id}>
                <Card className="flex h-full flex-col gap-3 p-4" data-testid="usage-account">
                  <span className="flex items-center gap-2 text-sm font-medium">
                    <span aria-hidden="true" className="size-2 rounded-full" style={{ background: accountColor(account.id) }} />
                    <ToolMark tool={account.provider} size={14} className="text-muted" />
                    {account.label}
                  </span>
                  <AccountRunway
                    display={displays.find((d) => d.account === account.id)}
                    sessions={sessions.filter((s) => s.account_id === account.id)}
                  />
                  <BurnDowns display={displays.find((d) => d.account === account.id)} />
                </Card>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section aria-labelledby="usage-costs" className="flex flex-col gap-2.5">
        <h3 id="usage-costs" className="text-[11px] font-semibold uppercase tracking-wider text-muted">
          What sessions used
        </h3>
        <CostMatrix />
      </section>
    </div>
  );
}
