/**
 * 0.4.6 (D-126): Who's in's Apps as logo cards, each ticked into the
 * team. A chosen desktop App that still needs AI Juggler's connector says
 * so right under the cards, in one line with Preview and add, which opens
 * the consent card's own preview (round 7: asked for before Next, one
 * component on both paths; D-126's mock).
 *
 * A card leads with what tells it apart (the rc3 critique: two "Claude
 * Code CLI" cards): its title is the App's short name, as the preview and
 * the canvas's planned cards say it (`shortNames`: "Work Max" and
 * "Personal" where two share a tool, a lone one its app word, "Claude");
 * under it, its tool and kind ("Claude Code CLI · Terminal"), its profile
 * when the title isn't it, and what's open in the pool's projects now.
 *
 * The cards sit in a dense grid: a chosen App's connector row spans the
 * grid's width and the next cards fill in above it, so the row reads as
 * the cards' footnote while it stays its App's (its own element, for
 * screen readers and the tests). The sessions open now are Who's in's
 * own section under them (NewPoolWho), each saying who brings it in.
 */
import { useId, useState, type JSX } from "react";

import { CheckCard } from "../../components/controls";
import { Button } from "../../components/primitives";
import { ShellOnlyChip } from "../../components/ShellOnly";
import type { AppModel } from "../../lib/apps";
import type { Candidate, Draft } from "../../lib/new-pool";
import { openNowByMember } from "../../lib/pool-open-now";
import { shortNames } from "../../lib/pool-values";
import { APP_KIND_WORDS } from "../../lib/pool-words";
import { sessionState } from "../../lib/rail-items";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { ConnectorNotice } from "../../pages/pool/connector/ConnectorNotice";
import { DesktopReachTag } from "../../pages/pool/connector/DesktopReach";
import { logoOf, MemberLogo } from "../../components/MemberLogo";

/** App cards shown at first; Show all adds the rest (instant and paged). */
export const APP_CARDS = 6;

/** What's open of an App in the pool's projects now: "1 open in web". */
export function openInWords(count: number, where: string): string {
  const place = where === "every project" ? "any project" : where;
  return count > 0 ? `${count} open in ${place}` : `Nothing open in ${place}`;
}

/** The sessions open now that a chosen App (or everyone) brings in,
 * each with who brings it. */
export function coveredSessions(draft: Pick<Draft, "every" | "projects" | "members">, sessions: readonly InstanceInfo[], apps: readonly AppModel[], everyone: string): { session: InstanceInfo; by: string }[] {
  const chosen = draft.members.filter((member) => member.kind === "app");
  const names = shortNames(chosen.map((member) => member.label));
  const all = draft.members.some((member) => member.ref === "all");
  return sessions
    .filter((session) => sessionState(session).state !== "ended" && (draft.every || draft.projects.includes(session.project_id ?? null)))
    .flatMap((session) => {
      const at = chosen.findIndex((member) => {
        const app = apps.find((one) => `app:${one.id}` === member.ref);
        return !!app?.folder && app.folder.id === session.account_id;
      });
      if (at >= 0) return [{ session, by: `In, with ${names[at]}` }];
      return all ? [{ session, by: `In, as ${everyone}` }] : [];
    });
}

/** A card's words under its title: its tool and kind ("Claude Code CLI ·
 * Terminal"; the tool left out where it is the title), then its profile
 * where the title isn't it. */
function appCardLines(title: string, app: AppModel): { sub: string; profile: string | null } {
  const sub = [app.app === title ? null : app.app, APP_KIND_WORDS[app.kind]].filter(Boolean).join(" · ");
  return { sub, profile: app.profile && app.profile !== title ? app.profile : null };
}

function AppCard({ row, title, app, on, open, where, onToggle }: { row: Candidate; title: string; app: AppModel | undefined; on: boolean; open: number | null; where: string; onToggle: () => void }): JSX.Element {
  const blocked = row.blocked !== null;
  const desktop = app?.kind === "desktop";
  const lines = app ? appCardLines(title, app) : null;
  return (
    // Its own element (its connector row goes in it), laid out by the grid.
    <div className="contents" data-testid="new-pool-candidate" data-ref={row.ref}>
      <CheckCard
        layout="stack"
        checked={on}
        onChange={onToggle}
        disabled={blocked}
        reason={row.blocked ?? undefined}
        logo={<MemberLogo spec={logoOf(row.ref, { apps: app ? [app] : [], sessions: [] })} size={40} />}
        title={title}
        sub={lines?.sub ?? row.detail}
      >
        {lines?.profile && <span className="text-label text-muted">Profile {lines.profile}</span>}
        <span className="text-label text-faint">{open === null ? row.detail : openInWords(open, where)}</span>
        {/* Where its app stands, in a word (the list says why, once). */}
        {!blocked && row.note && !on && (
          <span className="mt-1">
            <DesktopReachTag member={row.ref} testId="new-pool-candidate-reach" />
          </span>
        )}
        {!blocked && row.shell && (
          // Above the card's input (positioned, after it), so its hint
          // shows on hover; the card's words are its checkbox's
          // description (its face is shown, not read), its why with them.
          <span className="relative mt-1">
            <ShellOnlyChip why={row.shell} inRow />
            <span className="sr-only">. {row.shell}</span>
          </span>
        )}
        {blocked && <span className="text-label text-ink-2">{row.blocked}</span>}
      </CheckCard>
      {on && desktop && app && <ConnectorNotice appId={app.id} app={app.name} kind={app.app} profile={app.profile} testId="new-pool-connector" />}
    </div>
  );
}

export function NewPoolWhoApps({
  rows,
  draft,
  apps,
  sessions,
  where,
  onToggle,
}: {
  rows: readonly Candidate[];
  draft: Draft;
  apps: readonly AppModel[];
  /** What's open now (to count each App's in the pool's projects). */
  sessions: readonly InstanceInfo[] | undefined;
  where: string;
  onToggle: (row: Candidate) => void;
}): JSX.Element {
  const [all, setAll] = useState(false);
  const title = useId();
  const has = (ref: string) => draft.members.some((member) => member.ref === ref);
  // Chosen ones and the ones open now first; a page, then Show all.
  const order = [...rows].sort((a, b) => Number(has(b.ref)) - Number(has(a.ref)) || (a.group === "open" ? 0 : 1) - (b.group === "open" ? 0 : 1));
  const shown = all ? order : order.slice(0, APP_CARDS);
  const openOf = (row: Candidate) =>
    sessions ? (openNowByMember({ every: draft.every, projects: draft.projects, members: [{ ref: row.ref, kind: row.kind, label: row.label, role: "member", level: null, reach: row.reach }] }, sessions, { apps, subscriptions: [] }).get(row.ref) ?? 0) : null;
  // Every card by its short name among all of them, so two of one tool
  // are told apart whether or not both are chosen.
  const short = shortNames(
    rows.map((row) => row.label),
    rows.map((row) => row.ref),
  );
  const titles = new Map(rows.map((row, at) => [row.ref, short[at]!]));
  return (
    <section aria-labelledby={title} className="flex flex-col gap-3" data-testid="new-pool-apps">
      <h4 className="flex flex-wrap items-baseline gap-x-2.5">
        <span id={title} className="section-label">
          Your apps
        </span>
        <span className="text-label text-faint">Each one: every session it starts in {where} joins</span>
      </h4>
      {rows.length === 0 ? (
        <p className="text-meta text-ink-2">No apps yet.</p>
      ) : (
        <div className="grid grid-flow-row-dense grid-cols-1 gap-2.5 sm:grid-cols-3" data-testid="new-pool-app-cards">
          {shown.map((row) => (
            <AppCard key={row.ref} row={row} title={titles.get(row.ref) ?? row.label} app={apps.find((one) => `app:${one.id}` === row.ref)} on={has(row.ref)} open={openOf(row)} where={where} onToggle={() => onToggle(row)} />
          ))}
        </div>
      )}
      {order.length > shown.length && (
        <Button variant="ghost" size="sm" className="self-start" onClick={() => setAll(true)} data-testid="new-pool-apps-all">
          Show all {order.length} apps
        </Button>
      )}
    </section>
  );
}
