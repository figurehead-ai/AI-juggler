/**
 * AJ-E3007 step 2, AJ-E3011's "Who's in": who works together, picked from
 * what exists (wireframe 2). Where is its own step before it.
 *
 * - First, every new session in the pool's projects: a switch (it is a
 *   member, "everyone", that joins them by itself, now and later).
 * - D-126: then the Apps as logo cards (NewPoolWhoApps), a chosen desktop
 *   App's connector asked for under them; then the sessions open now in
 *   its projects, each with its check (in with its App or everyone, or
 *   ticked by itself), the earlier ones and those in other projects a
 *   fold under them; then More ways to add (Accounts, Tools, Macs), a
 *   fold closed unless the pool starts with one. The rc3 critique: the
 *   "Apps | Sessions | More" switch read as cryptic; every kind is on the
 *   page now, in the mock-up's order.
 * - Each list: Open now first, then Recent, then All; search past 8; each
 *   group paged. A row that can't join says why and offers the step that
 *   would let it in. Who's in is said by the preview beside, in words.
 * - Calm (D-117): surface tone, no row dividers; colour only for what
 *   needs you.
 */
import { useId, useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { Disclosure, RowList, ToggleRow } from "../../components/controls";
import { IconCheck } from "../../components/icons-controls";
import { IconSessions } from "../../components/icons";
import { ShellOnlyChip } from "../../components/ShellOnly";
import { logoOf, MemberLogo, type LogoSources } from "../../components/MemberLogo";
import type { MemberKind } from "../../lib/ipc/pool-model";
import { DesktopListNote, DesktopReachTag } from "../../pages/pool/connector/DesktopReach";
import { useApps, type AppModel } from "../../lib/apps";
import { EVERYONE, nextRole, REACH_WORDS, type Candidate, type Draft, type DraftMember } from "../../lib/new-pool";
import { MACHINE_WORD, MACHINES_WORD } from "../../lib/machine-words";
import { templateById } from "../../lib/pool-templates";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { coveredSessions, NewPoolWhoApps } from "./NewPoolWhoApps";
import { StepHead } from "./StepHead";

/** Past this many rows a list gets a search field (and All shows this
 * many at first). */
const SEARCH_PAST = 8;
/** Rows a group shows, and Show more adds (the desktop rule: a list that
 * grows is paged; rc27 live pass 2: 143 sessions were drawn at once). */
export const CANDIDATES_PAGE = 20;

const KIND_WORDS: Record<MemberKind, string> = {
  session: "Sessions",
  app: "Apps",
  account: "Accounts",
  tool: "Tools",
  mac: MACHINES_WORD,
  everyone: "Everyone",
};

/** What adding one of the wider kinds brings, in a line. */
const KIND_LINES: Partial<Record<MemberKind, string>> = {
  account: "An Account: every session of every App signed in to it joins.",
  tool: "A tool: every session of it joins, on any App or Account.",
  mac: `A ${MACHINE_WORD}: every session on it joins.`,
};

/** The wider kinds, under More ways to add (everyone is the switch). */
const MORE_KINDS: readonly MemberKind[] = ["account", "tool", "mac"];
const isMore = (kind: MemberKind | undefined) => kind !== undefined && MORE_KINDS.includes(kind);

const GROUPS: { id: Candidate["group"]; label: string }[] = [
  { id: "open", label: "Open now" },
  { id: "recent", label: "Recent" },
  { id: "all", label: "All" },
];

export function NewPoolWho({
  draft,
  onDraft,
  candidates,
  onFix,
  nameOf = (_, label) => label,
  where = "these projects",
  sessions,
  apps: given,
}: {
  draft: Draft;
  onDraft: (next: Draft) => void;
  candidates: Record<MemberKind, Candidate[]>;
  onFix: (candidate: Candidate) => void;
  /** A member's name on screen (everyone by its scope). */
  nameOf?: (ref: string, label: string) => string;
  /** Its projects in words ("web"). */
  where?: string;
  /** What's open now: each App's count here, and who brings each in. */
  sessions?: readonly InstanceInfo[];
  /** Every App, the CLI ones too (read with the folders). */
  apps?: readonly AppModel[] | null;
}): JSX.Element {
  const ids = useId();
  const has = (ref: string) => draft.members.some((member) => member.ref === ref);
  const toggle = (row: Candidate) => {
    if (has(row.ref)) onDraft({ ...draft, members: draft.members.filter((member) => member.ref !== row.ref) });
    else {
      const member: DraftMember = { ref: row.ref, kind: row.kind, label: row.label, role: nextRole(draft, row.kind, row.reach), level: null, reach: row.reach, shell: row.shell ?? null };
      onDraft({ ...draft, members: [...draft.members, member] });
    }
  };
  const read = useApps();
  const apps = given ?? read ?? [];
  const sources: LogoSources = { apps, sessions: sessions ?? [] };
  const everyone = draft.members.find((member) => member.ref === "all");
  const everyoneName = nameOf("all", EVERYONE.label);
  const setEveryone = (on: boolean) =>
    onDraft({ ...draft, members: on ? [...draft.members, { ...EVERYONE, role: templateById(draft.template).broadRole ?? "member" }] : draft.members.filter((member) => member.ref !== "all") });
  // The sessions open now in its projects; the rest (earlier ones, and
  // those that need their project added first) fold under them.
  const open = candidates.session.filter((row) => row.group === "open" && row.fix?.action !== "scope");
  const other = candidates.session.filter((row) => !open.includes(row));
  const by = new Map(sessions ? coveredSessions(draft, sessions, apps, everyoneName.charAt(0).toLowerCase() + everyoneName.slice(1)).map((one) => [`session:${one.session.id}`, one.by]) : []);
  // A pool that starts with an Account or a Mac, or has one, shows them.
  const moreOpen = isMore(templateById(draft.template).kinds[0]) || draft.members.some((member) => isMore(member.kind));
  const list = { has, onToggle: toggle, onFix, sources };
  return (
    <div className="flex min-w-0 flex-col gap-7" data-testid="new-pool-step-who">
      <StepHead step="who" />
      <RowList testId="new-pool-everyone">
        <ToggleRow
          icon={IconSessions}
          title={`Every new session in ${where}`}
          help={`Any session you start in ${where} joins by itself, now and later.${everyone ? " Already in." : ""}`}
          checked={!!everyone}
          onChange={setEveryone}
          testId="new-pool-everyone-switch"
        />
      </RowList>
      <NewPoolWhoApps rows={candidates.app} draft={draft} apps={apps} sessions={sessions} where={where} onToggle={toggle} />
      {/* Round 7 #9: how a desktop session takes part, once for the step
          (AJ-E3014 item 4: under the Apps and again in the sessions). */}
      <DesktopListNote notes={Object.values(candidates).flatMap((rows) => rows.map((row) => (row.blocked ? undefined : row.note)))} testId="new-pool-desktop-note" />
      <section aria-labelledby={`${ids}-open`} className="flex min-w-0 flex-col gap-2.5" data-testid="new-pool-sessions">
        <h4 className="flex items-baseline gap-2.5">
          <span id={`${ids}-open`} className="section-label">
            Sessions open now in {where}
          </span>
          <span className="text-label text-faint">{open.length}</span>
        </h4>
        <CandidateList rows={open} kind="session" search="Search sessions open now" none={`Nothing open in ${where} now.`} labelled={false} by={by} testId="new-pool-sessions-open" {...list} />
        {other.length > 0 && (
          <Disclosure
            summary={
              <span>
                Earlier sessions, or in another project <span className="text-faint">· {other.length}</span>
              </span>
            }
            testId="new-pool-sessions-other"
          >
            <div className="mt-2.5 flex flex-col gap-2.5">
              <CandidateList rows={other} kind="session" search="Search earlier sessions" testId="new-pool-sessions-earlier" {...list} />
            </div>
          </Disclosure>
        )}
      </section>
      <Disclosure
        open={moreOpen}
        summary={
          <span>
            More ways to add: a {MACHINE_WORD}, an account or a tool <span className="font-normal text-faint">· for bigger setups</span>
          </span>
        }
        tone="heading"
        testId="new-pool-more"
      >
        <div className="mt-3 flex flex-col gap-6">
          {MORE_KINDS.map((kind) => (
            <section key={kind} aria-labelledby={`${ids}-${kind}`} className="flex min-w-0 flex-col gap-2.5" data-testid={`new-pool-kind-${kind}`}>
              <h4 className="flex items-baseline gap-2.5">
                <span id={`${ids}-${kind}`} className="section-label">
                  {KIND_WORDS[kind]}
                </span>
                <span className="text-label text-faint">{candidates[kind].length}</span>
              </h4>
              <p className="text-meta text-ink-2">{KIND_LINES[kind]}</p>
              <CandidateList rows={candidates[kind]} kind={kind} search={`Search ${KIND_WORDS[kind].toLowerCase()}`} testId={`new-pool-${kind}-list`} {...list} />
            </section>
          ))}
        </div>
      </Disclosure>
    </div>
  );
}

/** A kind's rows by group (Open now, Recent, All), searched past 8, each
 * group paged. `labelled`: each group says its name (one group: none);
 * `by`: who brings a session in already ("In, with Work Max"). */
function CandidateList({
  rows: all,
  kind,
  search,
  none,
  labelled = true,
  by,
  testId,
  has,
  onToggle,
  onFix,
  sources,
}: {
  rows: readonly Candidate[];
  kind: MemberKind;
  /** Its search field's name. */
  search: string;
  /** What it says when it has no rows. */
  none?: string;
  labelled?: boolean;
  by?: ReadonlyMap<string, string>;
  testId: string;
  has: (ref: string) => boolean;
  onToggle: (row: Candidate) => void;
  onFix: (row: Candidate) => void;
  sources: LogoSources;
}): JSX.Element {
  const [query, setQuery] = useState("");
  // Show more's pages so far, by group (none: the first page).
  const [more, setMore] = useState<Partial<Record<Candidate["group"], number>>>({});
  const words = query.trim().toLowerCase();
  const rows = all.filter((row) => !words || `${row.label} ${row.detail}`.toLowerCase().includes(words));
  return (
    <>
      {all.length > SEARCH_PAST && (
        <input
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          type="search"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setMore({});
          }}
          placeholder={search}
          aria-label={search}
          className="h-9 rounded-[10px] bg-fill px-3 text-meta text-ink outline-none inset-ring-1 inset-ring-hairline/75 placeholder:text-muted focus-visible:inset-ring-[1.5px] focus-visible:inset-ring-accent focus-visible:ring-4 focus-visible:ring-accent-soft"
        />
      )}
      <div className="flex flex-col gap-4" data-testid={testId}>
        {GROUPS.map((group) => {
          const inGroup = rows.filter((row) => row.group === group.id);
          if (inGroup.length === 0) return null;
          const first = group.id === "all" && !query ? SEARCH_PAST : CANDIDATES_PAGE;
          const cut = inGroup.slice(0, first + (more[group.id] ?? 0) * CANDIDATES_PAGE);
          return (
            <div key={group.id} className="flex flex-col gap-1.5">
              {labelled && (
                <span className="flex items-baseline gap-2.5">
                  <span className="section-label">{group.label}</span>
                  <span className="text-label text-faint">{inGroup.length}</span>
                </span>
              )}
              <ul className="flex flex-col gap-0.5 rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
                {cut.map((row) => (
                  <CandidateRow key={row.ref} row={row} on={has(row.ref)} by={by?.get(row.ref)} sources={sources} onToggle={() => onToggle(row)} onFix={() => onFix(row)} />
                ))}
              </ul>
              {cut.length < inGroup.length && (
                <Button variant="ghost" size="sm" className="self-start" onClick={() => setMore((was) => ({ ...was, [group.id]: (was[group.id] ?? 0) + 1 }))} data-testid={`new-pool-more-${group.id}`}>
                  Show more ({inGroup.length - cut.length} more)
                </Button>
              )}
            </div>
          );
        })}
        {rows.length === 0 && <p className="text-meta text-ink-2">{query ? "Nothing matches." : (none ?? `No ${KIND_WORDS[kind].toLowerCase()} yet.`)}</p>}
      </div>
    </>
  );
}

/** One row: its check, its logo, its name and line, and what it brings
 * (its reach), or who brings it in already: then its check is the
 * mock-up's quiet one, and ticking it adds it by itself too (its own
 * role). */
function CandidateRow({ row, on, by, sources, onToggle, onFix }: { row: Candidate; on: boolean; by: string | undefined; sources: LogoSources; onToggle: () => void; onFix: () => void }): JSX.Element {
  const blocked = row.blocked !== null;
  const brought = !on && !blocked && by !== undefined;
  return (
    <li className={"flex flex-col gap-1 rounded-[10px] px-3 py-2 " + (on ? "bg-accent-soft" : blocked ? "" : "hover:bg-surface-2")} data-testid="new-pool-candidate" data-ref={row.ref} data-reach={row.reach} data-brought={brought || undefined}>
      <button
        type="button"
        role="checkbox"
        aria-checked={on}
        disabled={blocked}
        onClick={onToggle}
        className="flex min-w-0 items-center gap-3 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-default"
      >
        <span aria-hidden="true" className={"flex size-5 shrink-0 items-center justify-center rounded-md text-label " + (on ? "bg-accent-fill text-white" : brought ? "bg-surface-3 text-faint" : blocked ? "inset-ring-[1.5px] inset-ring-hairline" : "inset-ring-[1.5px] inset-ring-faint")}>
          {on || brought ? <IconCheck width={12} height={12} strokeWidth={3} /> : null}
        </span>
        <MemberLogo spec={logoOf(row.ref, sources)} size={24} />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className={"truncate text-body font-medium " + (blocked ? "text-ink-2" : "text-ink")}>{row.label}</span>
          <span className="truncate text-meta text-ink-2">{row.detail}</span>
        </span>
        {/* A row that can't join has no reach to promise (the P6 walk: a
            paired Mac said "new ones join" beside why it can't). */}
        {!blocked && <span className="shrink-0 text-meta text-muted">{brought ? by : REACH_WORDS[row.reach]}</span>}
        {/* Round 7 #9: where its app stands, in a word (the list says why). */}
        {!blocked && row.note && !on && <DesktopReachTag member={row.ref} testId="new-pool-candidate-reach" />}
        {!blocked && row.shell && <ShellOnlyChip why={row.shell} inRow />}
      </button>
      {(row.blocked || (row.fix && row.reach === "watched")) && (
        <span className="flex flex-wrap items-center gap-2 pl-[4.25rem] text-meta text-ink-2">
          {row.blocked ?? "Runs in another app: it can be watched, not act."}
          {row.fix && (
            <Button size="sm" variant="ghost" onClick={onFix} data-testid="new-pool-fix">
              {row.fix.label}
            </Button>
          )}
        </span>
      )}
    </li>
  );
}
