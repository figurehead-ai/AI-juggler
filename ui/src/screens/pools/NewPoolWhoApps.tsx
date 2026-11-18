/**
 * 0.4.6 (D-126): Who's in's Apps as logo cards — Desktop app or
 * Terminal, then the profile, then what's open in the pool's projects
 * now — each ticked into the team. A chosen desktop App that still
 * needs AI Juggler's connector says so right under the cards, on its own
 * row with the consent card's own Add (round 7: asked for before Next,
 * one component on both paths). Under them, the sessions open now that
 * the chosen Apps (or everyone) bring in, so nobody wonders.
 *
 * The cards sit in a dense grid: a chosen App's connector row spans the
 * grid's width and the next cards fill in above it, so the row reads as
 * the cards' footnote while it stays its App's (its own element, for
 * screen readers and the tests).
 */
import { useState, type JSX } from "react";

import { CheckCard } from "../../components/controls";
import { Button } from "../../components/primitives";
import { IconConnect } from "../../components/icons-menu";
import { IconCheck } from "../../components/icons-controls";
import { ShellOnlyChip } from "../../components/ShellOnly";
import type { AppModel } from "../../lib/apps";
import type { Candidate, Draft } from "../../lib/new-pool";
import { openNowByMember } from "../../lib/pool-open-now";
import { shortNames } from "../../lib/pool-values";
import { APP_KIND_WORDS } from "../../lib/pool-words";
import { sessionState } from "../../lib/rail-items";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { ConnectorCard } from "../../pages/pool/connector/ConnectorCard";
import { DesktopListNote, DesktopReachTag } from "../../pages/pool/connector/DesktopReach";
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

function AppCard({ row, app, on, open, where, onToggle }: { row: Candidate; app: AppModel | undefined; on: boolean; open: number | null; where: string; onToggle: () => void }): JSX.Element {
  const blocked = row.blocked !== null;
  const desktop = app?.kind === "desktop";
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
        title={app?.app ?? row.label}
        sub={app ? APP_KIND_WORDS[app.kind] : row.detail}
      >
        {app?.profile && <span className="text-label text-muted">Profile {app.profile}</span>}
        <span className="text-label text-faint">{open === null ? row.detail : openInWords(open, where)}</span>
        {/* Where its app stands, in a word (the list says why, once). */}
        {!blocked && row.note && !on && (
          <span className="mt-1">
            <DesktopReachTag member={row.ref} testId="new-pool-candidate-reach" />
          </span>
        )}
        {!blocked && row.shell && (
          <span className="mt-1">
            <ShellOnlyChip why={row.shell} inRow />
          </span>
        )}
        {blocked && <span className="text-label text-ink-2">{row.blocked}</span>}
      </CheckCard>
      {on && desktop && app && (
        <div className="col-span-full flex items-start gap-3 rounded-xl bg-surface-2 px-3.5 py-3" data-testid="new-pool-connector">
          <IconConnect aria-hidden="true" width={16} height={16} className="mt-0.5 shrink-0 text-faint" />
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <p className="text-meta font-medium text-ink">{app.name}</p>
            <ConnectorCard compact appId={app.id} app={app.name} kind={app.app} profile={app.profile} heading={4} testId="new-pool-connector-card" />
          </div>
        </div>
      )}
    </div>
  );
}

export function NewPoolWhoApps({
  rows,
  draft,
  apps,
  sessions,
  where,
  everyone,
  sessionRows = [],
  onToggle,
}: {
  rows: readonly Candidate[];
  /** The session candidates, for an open session's name and line. */
  sessionRows?: readonly Candidate[];
  draft: Draft;
  apps: readonly AppModel[];
  /** What's open now (to count each App's in the pool's projects). */
  sessions: readonly InstanceInfo[] | undefined;
  where: string;
  /** Everyone's name ("everyone in web"). */
  everyone: string;
  onToggle: (row: Candidate) => void;
}): JSX.Element {
  const [all, setAll] = useState(false);
  const has = (ref: string) => draft.members.some((member) => member.ref === ref);
  // Chosen ones and the ones open now first; a page, then Show all.
  const order = [...rows].sort((a, b) => Number(has(b.ref)) - Number(has(a.ref)) || (a.group === "open" ? 0 : 1) - (b.group === "open" ? 0 : 1));
  const shown = all ? order : order.slice(0, APP_CARDS);
  const openOf = (row: Candidate) =>
    sessions ? (openNowByMember({ every: draft.every, projects: draft.projects, members: [{ ref: row.ref, kind: row.kind, label: row.label, role: "member", level: null, reach: row.reach }] }, sessions, { apps, subscriptions: [] }).get(row.ref) ?? 0) : null;
  const covered = sessions ? coveredSessions(draft, sessions, apps, everyone) : [];
  return (
    <div className="flex flex-col gap-3" data-testid="new-pool-apps">
      <p className="flex flex-wrap items-baseline gap-x-2.5">
        <span className="section-label">Your apps</span>
        <span className="text-label text-faint">Each one: every session it starts in {where} joins</span>
      </p>
      {rows.length === 0 ? (
        <p className="text-meta text-ink-2">No apps yet.</p>
      ) : (
        <div className="grid grid-flow-row-dense grid-cols-1 gap-2.5 sm:grid-cols-3" data-testid="new-pool-candidates">
          {shown.map((row) => (
            <AppCard key={row.ref} row={row} app={apps.find((one) => `app:${one.id}` === row.ref)} on={has(row.ref)} open={openOf(row)} where={where} onToggle={() => onToggle(row)} />
          ))}
        </div>
      )}
      {order.length > shown.length && (
        <Button variant="ghost" size="sm" className="self-start" onClick={() => setAll(true)} data-testid="new-pool-apps-all">
          Show all {order.length} apps
        </Button>
      )}
      {/* Round 7 #9: how a desktop session takes part, once for the list. */}
      <DesktopListNote notes={rows.map((row) => (row.blocked ? undefined : row.note))} testId="new-pool-desktop-note" />
      {covered.length > 0 && (
        <section aria-labelledby="new-pool-covered-title" className="mt-3 flex flex-col gap-2.5" data-testid="new-pool-covered">
          <h4 id="new-pool-covered-title" className="flex items-baseline gap-2.5">
            <span className="section-label">Sessions open now in {where}</span>
            <span className="text-label text-faint">{covered.length}</span>
          </h4>
          <ul className="flex flex-col rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
            {covered.slice(0, 5).map(({ session, by }) => {
              const row = sessionRows.find((one) => one.ref === `session:${session.id}`);
              return (
              <li key={session.id} className="flex items-center gap-3 px-3 py-2">
                <MemberLogo spec={logoOf(`session:${session.id}`, { apps, sessions: [session] })} size={24} />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-body font-medium text-ink">{row?.label ?? session.title ?? session.id}</span>
                  <span className="truncate text-meta text-faint">{row?.detail ?? session.account_label}</span>
                </span>
                <span className="shrink-0 text-meta text-muted">{by}</span>
                <span aria-hidden="true" className="flex size-5 shrink-0 items-center justify-center rounded-md bg-surface-3 text-faint">
                  <IconCheck width={12} height={12} strokeWidth={3} />
                </span>
              </li>
              );
            })}
            {covered.length > 5 && <li className="px-3 py-2 text-meta text-muted">and {covered.length - 5} more</li>}
          </ul>
        </section>
      )}
    </div>
  );
}
