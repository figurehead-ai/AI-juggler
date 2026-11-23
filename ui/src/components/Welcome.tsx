/**
 * AJ-E1524, AJ-E1545 — the welcome Home for new installs. Home shows it
 * whenever there are no accounts, also after the setup's Not now. It
 * leads with "Let's get you set up" and one primary, Get started, which
 * opens the setup wizard (find your apps, the permissions, Meet Bot, keep
 * it running, the tour): nobody is left on their own. Below, what it does
 * for you — Bot first, full width, with its Beta tag (the maintainer's
 * amendment) — each card with a short clip of the effect (AJ-E1546), a
 * trust line, and "How it works" behind a quiet disclosure.
 *
 * The large mark is the app's real icon (`AppMark`); its one rise-in is
 * decorative and stops under reduced motion. Tool names come from the
 * catalog and the install probe (AJ-E1525), never from ids.
 */
import type { JSX } from "react";
import { useResource } from "../lib/use-resource";
import { Button, Card } from "./primitives";
import { Disclosure } from "./controls";
import { AppMark } from "./AppMark";
import { Clip } from "./Clip";
import { Pill } from "./Pill";
import { IconBot } from "./bot/bot-icons";
import { IconDashboard, IconProjects, IconSessions, IconShield, IconTerminal, IconUsage } from "./icons";
import { providersDiscover, type ProviderInstallData } from "../lib/ipc/providers";
import { useTools } from "../lib/tools";
import type { ClipId } from "../lib/clips";

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
    ? `We found ${listWords(ready.map((row) => row.display_name))} on this computer.`
    : "No AI coding tools found yet.";
}

/** Bot's card: what it does, in the maintainer's words (E38). */
const BOT_CARD = {
  title: "Bot, your AI assistant",
  text: "Your AI assistant for everything AI Juggler sees. Ask what needs you, what changed in a project or how much an account has left: every answer cites the sessions and records it comes from, and Bot never acts on its own.",
  clip: "what-bot" as ClipId,
};

const BENEFITS: ReadonlyArray<{ icon: typeof IconSessions; text: string; clip: ClipId }> = [
  { icon: IconSessions, text: "Every session across your accounts in one list, with what needs you first.", clip: "what-sessions" },
  { icon: IconUsage, text: "See how much each account has left, and carry on in another before you hit the wall.", clip: "what-usage" },
  { icon: IconDashboard, text: "What your Mac is running, and how hard it's working.", clip: "what-mac" },
  { icon: IconProjects, text: "Your setups used where they are. Nothing is copied unless you choose.", clip: "what-setups" },
];

const STEPS: ReadonlyArray<{ title: string; text: string; clip: ClipId }> = [
  { title: "Bring in your apps", text: "AI Juggler finds the setups already on this computer and uses each one where it is.", clip: "how-apps" },
  { title: "Start a session", text: "Pick a project and an app. The session runs in its own terminal, right here.", clip: "how-start" },
  { title: "Juggle", text: "See what needs you, and when an account runs low, carry on in another.", clip: "how-juggle" },
];

function BotCard(): JSX.Element {
  return (
    <Card className="grid items-center gap-5 p-5 sm:grid-cols-[minmax(0,1fr)_minmax(0,300px)]" data-testid="welcome-bot">
      <div className="flex min-w-0 flex-col gap-2.5">
        <span className="flex items-center gap-3">
          <span aria-hidden="true" className="flex size-9 shrink-0 items-center justify-center rounded-[10px] bg-accent-soft text-accent-hi">
            <IconBot width={20} height={20} />
          </span>
          <span className="text-section font-semibold text-ink">{BOT_CARD.title}</span>
          <Pill tone="neutral">Beta</Pill>
        </span>
        <p className="text-body text-ink-2">{BOT_CARD.text}</p>
      </div>
      <Clip id={BOT_CARD.clip} label={BOT_CARD.title} />
    </Card>
  );
}

export function Welcome({ onGetStarted, onFreshAccount }: { onGetStarted: () => void; onFreshAccount: () => void }): JSX.Element {
  const tools = useTools();
  const discovered = useResource("providers-discover", providersDiscover);
  const installs: ProviderInstallData[] | null = discovered.error !== undefined ? [] : (discovered.data ?? null);
  // The lead line claims "runs your … sessions": only tools walked live
  // end to end (their adapters name the evidence), never one proven on
  // fixtures alone (D-127; the setup still brings every built tool in).
  const names = tools.filter((tool) => tool.flow.first_run && tool.flow.live_verified).map((tool) => tool.display_name);
  const toInstall = installs === null || installedTools(installs).length > 0 ? [] : installs.filter((row) => row.install_url);
  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-9 px-1 py-10 sm:py-14" data-testid="home-empty-accounts">
      <header className="relative flex flex-col items-start gap-5">
        <span aria-hidden="true" className="welcome-rise relative flex drop-shadow-[0_8px_24px_var(--color-accent-soft)]">
          <AppMark size={80} />
        </span>
        <div className="relative flex flex-col gap-3">
          <h1 className="text-stat font-semibold leading-tight tracking-tight">Let's get you set up</h1>
          <p className="max-w-2xl text-section leading-relaxed text-ink-2">
            AI Juggler runs your {names.length > 0 ? listWords(names) : "AI coding"} sessions side by side, shows you what needs you, and
            helps you carry on in another account before one runs out. Setup takes about two minutes, and every step can wait.
          </p>
        </div>
        <output
          className="flex min-h-7 flex-wrap items-center gap-x-2 gap-y-1 rounded-full border border-hairline bg-surface px-3 py-1 text-meta text-ink"
          data-testid="welcome-found"
        >
          <span aria-hidden="true" className="text-accent-hi">
            <IconTerminal width={14} height={14} />
          </span>
          {installs === null ? "Looking at what's on this computer…" : foundWords(installs)}
          {toInstall.map((row) => (
            <a key={row.provider} href={row.install_url} target="_blank" rel="noreferrer" className="text-accent underline-offset-2 hover:underline">
              Install {row.display_name} →
            </a>
          ))}
        </output>
        <div className="flex flex-wrap gap-2.5">
          <Button variant="primary" onClick={onGetStarted} data-testid="welcome-get-started">
            Get started
          </Button>
          <Button variant="secondary" onClick={onFreshAccount} data-testid="welcome-fresh">
            Add a fresh app
          </Button>
        </div>
      </header>

      <section aria-labelledby="welcome-what" className="flex flex-col gap-3">
        <h2 id="welcome-what" className="text-section font-semibold text-ink">
          What it does for you
        </h2>
        <BotCard />
        <ul className="grid gap-2.5 sm:grid-cols-2">
          {BENEFITS.map(({ icon: Icon, text, clip }) => (
            <li key={text}>
              <Card className="flex h-full flex-col gap-3 p-4" data-testid="welcome-benefit">
                <Clip id={clip} label={text} />
                <span className="flex items-start gap-3">
                  <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent-hi">
                    <Icon width={16} height={16} />
                  </span>
                  <span className="text-body text-ink">{text}</span>
                </span>
              </Card>
            </li>
          ))}
        </ul>
        <p className="flex items-center gap-2 text-meta text-ink-2" data-testid="welcome-trust">
          <span aria-hidden="true" className="text-success">
            <IconShield width={14} height={14} />
          </span>
          AI Juggler never stores your passwords or keys, and never sits between you and the tool.
        </p>
      </section>

      <Disclosure className="text-body" summary={<span className="font-medium">How it works</span>} testId="welcome-how">
        <ol className="mt-4 grid gap-4 sm:grid-cols-3">
          {STEPS.map((step, index) => (
            <li key={step.title} className="flex flex-col gap-2.5">
              <Clip id={step.clip} label={step.title} />
              <span className="flex items-start gap-3">
                <span aria-hidden="true" className="flex size-6 shrink-0 items-center justify-center rounded-full bg-surface-3 text-label font-semibold text-ink">
                  {index + 1}
                </span>
                <span className="flex flex-col gap-0.5">
                  <span className="font-medium text-ink">{step.title}</span>
                  <span className="text-meta text-muted">{step.text}</span>
                </span>
              </span>
            </li>
          ))}
        </ol>
      </Disclosure>
    </div>
  );
}

/** AJ-E1524: the one line Usage's and Fleet's empty states carry back to
 * the welcome while no account is in. */
export function WelcomePointer({ onOpenHome }: { onOpenHome: () => void }): JSX.Element {
  return (
    <p className="text-label text-muted" data-testid="welcome-pointer">
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
