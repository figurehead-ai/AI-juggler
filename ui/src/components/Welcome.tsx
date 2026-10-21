/**
 * AJ-E1524 — a welcome Home for new installs. Home shows it whenever
 * there are no accounts, also after first run's Not now: what AI Juggler
 * is for, what it found on this Mac, four things it does for you, a trust
 * line, and the way in — Find my accounts (first run) or Add a fresh
 * account. "How it works" waits behind a quiet disclosure. Tool names
 * come from the catalog and the install probe (AJ-E1525), never from ids.
 * The mark's one rise-in is decorative and stops under reduced motion.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { IconDashboard, IconJuggler, IconProjects, IconSessions, IconShield, IconTerminal, IconUsage } from "./icons";
import { providersDiscover, type ProviderInstallData } from "../lib/bridge";
import { useTools } from "../lib/tools";

/** "Claude Code", "Claude Code and Codex", "A, B and C". */
export function listWords(names: string[]): string {
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** The tools the probe found installed. */
export function installedTools(rows: ProviderInstallData[]): ProviderInstallData[] {
  return rows.filter((row) => row.status === "installed" || row.status === "unverified");
}

/** The scan's line, in words. */
export function foundWords(rows: ProviderInstallData[]): string {
  const ready = installedTools(rows);
  return ready.length > 0
    ? `We found ${listWords(ready.map((row) => row.display_name))} on this Mac.`
    : "No AI coding tools found yet.";
}

const BENEFITS = [
  { icon: IconSessions, text: "Every session across your accounts in one list, with what needs you first." },
  { icon: IconUsage, text: "See how much each account has left, and carry on in another before you hit the wall." },
  { icon: IconDashboard, text: "What your Mac is running, and how hard it's working." },
  { icon: IconProjects, text: "Your setups used where they are. Nothing is copied unless you choose." },
] as const;

const STEPS = [
  { title: "Bring in your accounts", text: "AI Juggler finds the setups already on this Mac and uses each one where it is." },
  { title: "Start a session", text: "Pick a project and an account. The session runs in its own terminal, right here." },
  { title: "Juggle", text: "See what needs you, and when an account runs low, carry on in another." },
] as const;

export function Welcome({
  onFindAccounts,
  onFreshAccount,
}: {
  onFindAccounts: () => void;
  onFreshAccount: () => void;
}): JSX.Element {
  const tools = useTools();
  const [installs, setInstalls] = useState<ProviderInstallData[] | null>(null);
  useEffect(() => {
    let alive = true;
    void providersDiscover().then(
      (rows) => alive && setInstalls(rows),
      () => alive && setInstalls([]),
    );
    return () => {
      alive = false;
    };
  }, []);
  // The tools first run brings in, by name, for the lead line.
  const names = tools.filter((tool) => tool.flow.first_run).map((tool) => tool.display_name);
  const toInstall = installs === null || installedTools(installs).length > 0 ? [] : installs.filter((row) => row.install_url);
  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-9 px-1 py-10 sm:py-14" data-testid="home-empty-accounts">
      <header className="relative flex flex-col items-start gap-5">
        <span
          aria-hidden="true"
          className="welcome-rise relative flex size-[72px] items-center justify-center rounded-2xl border border-accent/25 bg-surface shadow-glow-accent sm:size-20"
        >
          <IconJuggler width={64} height={64} />
        </span>
        <div className="relative flex flex-col gap-3">
          <h1 className="text-[28px] font-semibold leading-tight tracking-tight sm:text-[34px]">
            All your AI coding accounts, in one calm place.
          </h1>
          <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
            AI Juggler runs your {names.length > 0 ? listWords(names) : "AI coding"} sessions side by side, shows you what needs
            you, and helps you carry on in another account before one runs out.
          </p>
        </div>
        <p
          className="flex min-h-7 flex-wrap items-center gap-x-2 gap-y-1 rounded-full border border-hairline bg-surface px-3 py-1 text-[13px] text-ink"
          role="status"
          data-testid="welcome-found"
        >
          <span aria-hidden="true" className="text-accent-hi">
            <IconTerminal width={14} height={14} />
          </span>
          {installs === null ? "Looking at what's on this Mac…" : foundWords(installs)}
          {toInstall.map((row) => (
            <a
              key={row.provider}
              href={row.install_url}
              target="_blank"
              rel="noreferrer"
              className="text-accent underline-offset-2 hover:underline"
            >
              Install {row.display_name} →
            </a>
          ))}
        </p>
        <div className="flex flex-wrap gap-2.5">
          <Button variant="primary" onClick={onFindAccounts} data-testid="welcome-find">
            Find my accounts
          </Button>
          <Button variant="secondary" onClick={onFreshAccount} data-testid="welcome-fresh">
            Add a fresh account
          </Button>
        </div>
      </header>

      <section aria-labelledby="welcome-what" className="flex flex-col gap-3">
        <h2 id="welcome-what" className="text-[11px] font-semibold uppercase tracking-wider text-muted">
          What it does for you
        </h2>
        <ul className="grid gap-2.5 sm:grid-cols-2">
          {BENEFITS.map(({ icon: Icon, text }) => (
            <li key={text}>
              <Card className="flex h-full items-start gap-3 p-4">
                <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-accent/20 bg-accent-soft text-accent-hi">
                  <Icon width={16} height={16} />
                </span>
                <span className="text-sm leading-relaxed text-ink">{text}</span>
              </Card>
            </li>
          ))}
        </ul>
        <p className="flex items-center gap-2 text-[13px] text-muted" data-testid="welcome-trust">
          <span aria-hidden="true" className="text-success">
            <IconShield width={14} height={14} />
          </span>
          AI Juggler never stores your passwords or keys, and never sits between you and the tool.
        </p>
      </section>

      <details className="group text-sm" data-testid="welcome-how">
        <summary className="w-fit cursor-pointer rounded-md text-[13px] font-medium text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
          How it works
        </summary>
        <ol className="mt-4 flex flex-col gap-3">
          {STEPS.map((step, index) => (
            <li key={step.title} className="flex items-start gap-3">
              <span aria-hidden="true" className="flex size-6 shrink-0 items-center justify-center rounded-full border border-hairline bg-surface-2 text-[11px] font-semibold text-ink">
                {index + 1}
              </span>
              <span className="flex flex-col gap-0.5">
                <span className="font-medium text-ink">{step.title}</span>
                <span className="text-[13px] leading-relaxed text-muted">{step.text}</span>
              </span>
            </li>
          ))}
        </ol>
      </details>
    </div>
  );
}

/** AJ-E1524: the one line Usage's and Fleet's empty states carry back to
 * the welcome while no account is in. */
export function WelcomePointer({ onOpenHome }: { onOpenHome: () => void }): JSX.Element {
  return (
    <p className="text-[12px] text-muted" data-testid="welcome-pointer">
      New here?{" "}
      <button
        type="button"
        onClick={onOpenHome}
        className="rounded text-accent underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        Home shows what AI Juggler does and how to start
      </button>
      .
    </p>
  );
}
