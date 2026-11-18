/**
 * AJ-E3011 round 4 (main's ruling), D-30: New pool's own step for the one
 * question, before Review, while it's unanswered. Once answered here, it
 * says where they stand.
 *
 * D-126: drawn — two of the team's apps talking through AI Juggler —
 * then what it gives, what it costs and what it changes, side by side,
 * and the two answers as cards. The words and the answers are the pool
 * page's own question's (TEAM_TOOLS; paths-parity holds them alike); an
 * answer counts at once, as there. Answered Not now (or not able to run
 * here), the shared quiet line with its one fix says so.
 */
import type { JSX, SVGProps } from "react";

import { IconCheck } from "../../components/icons-controls";
import { IconKey, IconPool, IconRemove } from "../../components/icons-actions";
import { IconMessage } from "../../components/icons-canvas";
import { IconSessions } from "../../components/icons";
import { LogoTile } from "../../components/ToolLogo";
import { CARD_FOCUS, CHOSEN_CARD } from "../../components/controls";
import { agentBridgeAnswer } from "../../lib/ipc/agent-bridge";
import { AGENT_ACCESS, TEAM_TOOLS, openAgentAccess, teamToolsLine } from "../../lib/team-tools";
import { TeamToolsQuestion, useTeamTools, useToolsSave } from "../../pages/pool/setup/TeamTools";
import { ToolsArt } from "./illustrations";
import { logoOf, MemberLogo, type LogoSpec } from "./MemberLogo";
import { useApps } from "../../lib/apps";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { StepHead } from "./StepHead";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

/** Two of the team's apps, talking through AI Juggler. */
function Talking({ logos }: { logos: readonly LogoSpec[] }): JSX.Element {
  const end = (spec: LogoSpec | undefined, left: string) => (
    <span className="absolute top-[47%] -translate-x-1/2 -translate-y-1/2" style={{ left }}>
      {spec ? <MemberLogo spec={spec} size={40} /> : <LogoTile icon={IconSessions} size={40} />}
    </span>
  );
  return (
    <div aria-hidden="true" className="flex justify-center bg-canvas-hi bg-[radial-gradient(var(--color-hairline)_1px,transparent_1.2px)] bg-[length:14px_14px] px-4 pb-3 pt-4">
      <div className="relative aspect-[420/110] w-full max-w-[420px]">
        <span className="absolute inset-0">
          <ToolsArt />
        </span>
        {end(logos[0], "16.2%")}
        <span className="absolute left-1/2 top-[47%] -translate-x-1/2 -translate-y-1/2 text-accent-hi">
          <IconPool width={30} height={30} />
        </span>
        {end(logos[1], "83.8%")}
      </div>
    </div>
  );
}

function Fact({ icon: Shape, children }: { icon: Icon; children: string }): JSX.Element {
  return (
    <p className="flex flex-col gap-2 px-4 py-3.5 text-meta text-ink-2 sm:border-l sm:border-divider sm:first:border-l-0">
      <LogoTile icon={Shape} size={24} />
      {children}
    </p>
  );
}

/** An answer, as a card that answers at once. */
function Answer({ icon, words, on, disabled, onClick, testId }: { icon: Icon; words: string; on: boolean; disabled: boolean; onClick: () => void; testId: string }): JSX.Element {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={"flex min-w-0 items-center gap-3.5 rounded-card bg-surface px-4 py-3.5 text-left text-body font-semibold text-ink inset-ring-1 inset-ring-hairline/75 transition-shadow hover:inset-ring-faint/60 disabled:opacity-60 " + CARD_FOCUS}
      data-testid={testId}
    >
      <LogoTile icon={icon} size={32} on={on} />
      {words}
    </button>
  );
}

/** D-30's question, unanswered: the same words and answers as the pool
 * page's (TeamToolsQuestion), as cards. */
function Ask({ logos, onAnswer }: { logos: readonly LogoSpec[]; onAnswer?: () => void }): JSX.Element {
  const { busy, error, run } = useToolsSave();
  const answer = (yes: boolean) => {
    onAnswer?.();
    run(() => agentBridgeAnswer(yes));
  };
  return (
    <section aria-labelledby="new-pool-tools-title" className="flex flex-col gap-3.5" data-testid="new-pool-tools">
      <h4 id="new-pool-tools-title" className="sr-only">
        {TEAM_TOOLS.question}
      </h4>
      <div className="overflow-hidden rounded-card bg-surface inset-ring-1 inset-ring-hairline/75">
        <Talking logos={logos} />
        <div className="grid grid-cols-1 sm:grid-cols-3">
          <Fact icon={IconMessage}>{TEAM_TOOLS.gives}</Fact>
          <Fact icon={IconSessions}>{TEAM_TOOLS.cost}</Fact>
          <Fact icon={IconKey}>{TEAM_TOOLS.local}</Fact>
        </div>
        <p className="border-t border-divider px-4 py-3 text-meta text-muted">
          {TEAM_TOOLS.where}{" "}
          <button type="button" onClick={openAgentAccess} className="rounded font-medium text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="new-pool-tools-settings">
            {AGENT_ACCESS}
          </button>
          .
        </p>
      </div>
      <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
        <Answer icon={IconCheck} words={TEAM_TOOLS.yes} on disabled={busy} onClick={() => answer(true)} testId="new-pool-tools-yes" />
        <Answer icon={IconRemove} words={TEAM_TOOLS.notNow} on={false} disabled={busy} onClick={() => answer(false)} testId="new-pool-tools-not-now" />
      </div>
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

export function NewPoolTools({ onAnswer, members = [], open = [] }: { onAnswer?: () => void; /** The team's members (two of their apps are drawn talking). */ members?: readonly string[]; open?: readonly InstanceInfo[] }): JSX.Element {
  const state = useTeamTools();
  const apps = useApps() ?? [];
  const logos = members
    .filter((ref) => ref !== "all")
    .map((ref) => logoOf(ref, { apps, sessions: open }))
    .filter((spec) => spec.kind === "tool")
    .slice(0, 2);
  return (
    <div className="flex flex-col gap-7" data-testid="new-pool-step-tools">
      <StepHead step="tools" />
      {state?.kind === "ask" ? <Ask logos={logos} onAnswer={onAnswer} /> : <TeamToolsQuestion testId="new-pool-tools" onAnswer={onAnswer} />}
      {state?.kind === "on" && (
        <div className={"flex items-center gap-3.5 rounded-card px-4 py-3.5 " + CHOSEN_CARD}>
          <LogoTile icon={IconCheck} size={32} on />
          <p className="text-body font-semibold text-ink" data-testid="new-pool-tools-answered">
            {teamToolsLine(state)}
          </p>
        </div>
      )}
    </div>
  );
}
