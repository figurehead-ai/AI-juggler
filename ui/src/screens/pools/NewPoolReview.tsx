/**
 * AJ-E3007 step 4, AJ-E3011's Review (§1, §2): the whole pool read back
 * in sentences — its name and where, who's in, who leads, who builds and
 * who checks, who may message whom, what they may do on their own, the
 * seal and the team tools (with Turn on beside them while they're off) —
 * then what each member can do here, and what happens next. Round 3: the
 * sentences are the pool inspector's own (lib/pool-sentences), from one
 * shape, so the two paths can't say a pool differently. Make pool
 * follows.
 *
 * D-126: a state card first — "Everything it needs is set", or what
 * still keeps it from working (round 6: said before it's made), with
 * Turn on for the team tools — then the pool in one card, a short
 * sentence a row as mocked ("Codex checks the work. Everyone else
 * builds.", the steps' own values), each with its icon and Change; every
 * rule in full (the canvas's sentences, paths-parity's) one click away
 * under it; and what happens next as three cards. The first thing to
 * tell the lead is the side's (LeadPrompt).
 */
import type { JSX, ReactNode, SVGProps } from "react";

import { Alert } from "../../components/Alert";
import { Disclosure } from "../../components/controls";
import { IconCheck } from "../../components/icons-controls";
import { LogoTile } from "../../components/ToolLogo";
import { useApps } from "../../lib/apps";
import { desktopAppsOf, isAdded, useConnectorStatuses } from "../../lib/connector";
import type { Draft, Reach } from "../../lib/new-pool";
import { roleCard, type SetupStep } from "../../lib/pool-words";
import { openNowWords } from "../../lib/pool-open-now";
import { everyoneLines, leadLine, nextParts, ownLine, sealLine, shapeOfDraft, talkSentence, whoSentence, type PoolShape } from "../../lib/pool-sentences";
import { listWords } from "../../lib/account-terms";
import { reviewLines, whoLine } from "../../lib/pool-values";
import { reviewTitle } from "../../lib/pool-wizard-words";
import { draftPlaces, type TeamTools, type ToolsPlace } from "../../lib/team-tools";
import { TeamToolsFix, useTeamTools } from "../../pages/pool/setup/TeamTools";
import { SetupBlockers } from "../../pages/pool/setup/SetupBlockers";
import { ROW_WORDS, type PoolRow } from "./rows";
import { StepHead } from "./StepHead";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

const CAN: Record<Reach, string> = {
  full: "sees · messages · tasks · can be asked",
  shell: "acts through its shell: the juggler-agent command",
  watched: "through its app's AI Juggler connector; Watched without it",
  later: "its sessions here join by themselves, now and later",
};

type Row = PoolRow;

/** Each row (its title and icon the preview's too) and the step its
 * Change goes back to; the team tools are the state card's. */
const ROWS: readonly { row: Row; title: string; icon: Icon; step?: SetupStep }[] = (
  [
    ["where", "where"],
    ["who", "who"],
    ["lead", "lead"],
    ["everyone", "everyone"],
    ["talk", "talk"],
    ["own", "own"],
    ["sealed", "own"],
    ["tools", undefined],
  ] as const
).map(([row, step]) => ({ row, ...ROW_WORDS[row], step }));

/** A row's words in full: the pool inspector's own sentences. */
function words(row: Row, shape: PoolShape, where: string, name: string, tools: TeamTools | null): ReactNode {
  switch (row) {
    case "where":
      return `«${name}» works in ${where}.`;
    case "who":
      return whoSentence(shape);
    case "lead":
      return leadLine(shape, where);
    case "everyone": {
      const [join, ...each] = everyoneLines(shape, where);
      return (
        <>
          {join}
          {each.map((line) => (
            <span key={line} className="block" data-testid="new-pool-review-role">
              {line}
            </span>
          ))}
        </>
      );
    }
    case "talk":
      return talkSentence(shape);
    case "own":
      return ownLine(shape);
    case "sealed":
      return sealLine(shape);
    default:
      // Round 4: never "can't message" without its Turn on beside it.
      return tools ? <TeamToolsFix state={tools} testId="new-pool-review-tools" /> : "Reading…";
  }
}

/** First: everything it needs is set, or what still keeps it from
 * working (the team tools; a desktop App's connector). */
function StateCard({ draft, tools }: { draft: Draft; tools: TeamTools | null }): JSX.Element {
  const apps = useApps();
  const desktop = desktopAppsOf(
    draft.members.map((member) => member.ref),
    apps ?? [],
  );
  const statuses = useConnectorStatuses(desktop.map((app) => app.id));
  const connected = desktop.filter((app) => isAdded(statuses.get(app.id)?.state));
  const set = tools?.kind === "on" && connected.length === desktop.length;
  if (set) {
    const added = connected.length === 0 ? "" : `, and ${listWords(connected.map((app) => `${app.name}'s`))} connector ${connected.length === 1 ? "is" : "are"} added`;
    return (
      <div className="flex items-center gap-3.5 rounded-card bg-[color-mix(in_srgb,var(--color-accent)_9%,var(--color-surface))] px-4 py-3.5 inset-ring-[1.5px] inset-ring-accent" data-testid="new-pool-review-state" data-state="set">
        <span aria-hidden="true" className="flex size-9 shrink-0 items-center justify-center rounded-[10px] bg-accent-fill text-white">
          <IconCheck width={18} height={18} strokeWidth={2.4} />
        </span>
        <span className="flex min-w-0 flex-col">
          <span className="text-body font-semibold text-ink">Everything it needs is set</span>
          <span className="text-meta text-ink-2">Team tools are on{added}. Nothing is waiting on you.</span>
        </span>
      </div>
    );
  }
  return (
    <Alert level="needs-you" stack title="Before it can work" testId="new-pool-review-state" data-state="needs">
      {/* Never "can't message" without its Turn on beside it (round 4). */}
      {tools && tools.kind !== "on" && tools.kind !== "missing" && tools.kind !== "unsafe" && <TeamToolsFix state={tools} testId="new-pool-review-state-tools" />}
      {/* Round 6: what can't work yet, before it's made. */}
      <SetupBlockers refs={draft.members.map((member) => member.ref)} tools={tools} testId="new-pool-review-blocked" />
    </Alert>
  );
}

export function NewPoolReview({
  draft,
  name,
  where,
  nameOf = (_, label) => label,
  openNow,
  projects = [],
  onChange,
}: {
  draft: Draft;
  /** The name it gets (typed, else the suggestion). */
  name: string;
  /** Its projects in words ("web", "every project"). */
  where: string;
  nameOf?: (ref: string, label: string) => string;
  /** Each member's open sessions here now (`openNowByMember`): a broad
   * member says them as its canvas card will (the rc27 P1 walk). */
  openNow?: ReadonlyMap<string, number>;
  /** The projects by name (the team tools say which are off). */
  projects?: readonly ToolsPlace[];
  /** Back to a step to change it (each sentence's Change). */
  onChange?: (step: SetupStep) => void;
}): JSX.Element {
  const shape = shapeOfDraft(draft, nameOf);
  const short = reviewLines(shape, where, name);
  const places = draftPlaces(draft, projects);
  const tools = useTeamTools(places.projects, places.apps);
  const named = draft.members.filter((member) => member.ref !== "all");
  const [first, ...rest] = nextParts(shape, where);
  const upper = (line: string) => line.charAt(0).toUpperCase() + line.slice(1);
  const lead = shape.members.some((member) => member.role === "lead") || shape.leadNext !== null;
  // Who it opens with, by the steps' short names ("Claude and Codex, and
  // every new session in web").
  const team = draft.members.length > 0 ? whoLine(shape, where) : "";
  return (
    <div className="flex flex-col gap-6" data-testid="new-pool-step-review">
      <StepHead step="review" title={reviewTitle(name)} />
      <StateCard draft={draft} tools={tools} />
      <dl className="flex flex-col rounded-card bg-surface py-1 inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-review-rows">
        {ROWS.filter(({ row }) => row !== "tools").map(({ row, title, icon, step }) => (
          <div key={row} className="grid grid-cols-[28px_minmax(0,1fr)_auto] items-start gap-x-3.5 gap-y-0.5 border-t border-divider px-4 py-2.5 first:border-t-0 sm:grid-cols-[28px_7rem_minmax(0,1fr)_auto]" data-row={row}>
            <LogoTile icon={icon} size={24} />
            <dt className="pt-0.5 text-meta text-muted">{title}</dt>
            <dd className="col-start-2 pt-0.5 text-body text-ink sm:col-start-3" data-testid="new-pool-review-line">
              {row === "tools" ? null : short[row]}
            </dd>
            {onChange && step && (
              <dd className="col-start-3 row-start-1 sm:col-start-4">
                <button type="button" onClick={() => onChange(step)} aria-label={`Change ${title.toLowerCase()}`} className="rounded-md px-1.5 py-0.5 text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="new-pool-review-change">
                  Change
                </button>
              </dd>
            )}
          </div>
        ))}
      </dl>
      {/* Every rule in full, the canvas's own sentences, one click away. */}
      <Disclosure summary="Every rule in full" testId="new-pool-review-full" className="-mt-2">
        <dl className="mt-2 flex flex-col gap-1.5 rounded-xl bg-surface-2 px-4 py-3" data-testid="new-pool-review">
          {ROWS.map(({ row, title }) => (
            <div key={row} className="grid grid-cols-1 gap-x-4 gap-y-0.5 sm:grid-cols-[7rem_minmax(0,1fr)]" data-row={row}>
              <dt className="text-meta text-muted">{title}</dt>
              <dd className="text-meta text-ink-2">{words(row, shape, where, name, tools)}</dd>
            </div>
          ))}
        </dl>
      </Disclosure>
      {named.length > 0 && (
        <section className="flex flex-col gap-2" aria-label="What each member can do here">
          <span className="section-label">What each member can do here</span>
          <ul className="flex flex-col gap-0.5">
            {named.map((member) => (
              <li key={member.ref} className="flex flex-wrap items-baseline gap-x-3 gap-y-0.5 rounded-lg bg-surface-2 px-3 py-2">
                <span className="min-w-0 flex-1 truncate text-body font-medium text-ink">{nameOf(member.ref, member.label)}</span>
                <span className="text-meta text-ink-2">{member.ref === draft.leadNext ? "Its next session leads" : roleCard(member.role).label}</span>
                <span className="text-meta text-muted">{member.reach === "later" && openNow ? openNowWords(openNow.get(member.ref) ?? 0, where) : CAN[member.reach]}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      {/* Round 4 (#3): what happens once it's made, the diagram's own line, a card a part. */}
      <section aria-labelledby="new-pool-next-title" className="flex flex-col gap-3" data-testid="new-pool-review-next">
        <h4 id="new-pool-next-title" className="section-label">
          What happens next
        </h4>
        <ol className="grid grid-cols-1 gap-2.5 sm:grid-cols-3">
          {[
            { title: "The pool is made", line: team ? `Its diagram opens with ${team}.` : "Its diagram opens, ready for its members." },
            { title: lead ? "Start the lead" : "Open a session", line: `${upper(first!)}.` },
            { title: "Tell it the goal", line: `${upper(rest.join("; "))}.` },
          ].map((card, at) => (
            <li key={card.title} className="flex flex-col gap-1.5 rounded-xl bg-surface-2 p-3.5" data-testid="new-pool-review-next-step">
              <span className="flex items-center gap-2">
                <span aria-hidden="true" className="flex size-[22px] shrink-0 items-center justify-center rounded-full bg-accent-soft text-label font-semibold text-accent-hi">
                  {at + 1}
                </span>
                <span className="text-meta font-semibold text-ink">{card.title}</span>
              </span>
              <span className="text-meta text-ink-2">{card.line}</span>
            </li>
          ))}
        </ol>
      </section>
    </div>
  );
}
