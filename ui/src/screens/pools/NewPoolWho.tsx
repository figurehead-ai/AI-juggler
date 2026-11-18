/**
 * AJ-E3007 step 2, AJ-E3011's "Who's in": who works together, picked from
 * what exists (wireframe 2). Where is its own step before it.
 *
 * - First, every new session in the pool's projects: a switch (it is a
 *   member, "everyone", that joins them by itself, now and later).
 * - Then by kind: Apps (every session they start here joins) and single
 *   sessions, the rarer kinds (Accounts, Tools, Macs) under More; each
 *   kind says in a line what adding one brings. Open now first, then
 *   Recent, then All; search past 8; each list paged. A row that can't
 *   join says why and offers the step that would let it in.
 * - D-126: the Apps as logo cards (NewPoolWhoApps), a chosen desktop
 *   App's connector asked for under them; who's in is said by the
 *   preview beside, in words, as Review will.
 * - Calm (D-117): surface tone, no row dividers; colour only for what
 *   needs you.
 */
import { useMemo, useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { Picker, RowList, Segmented, ToggleRow } from "../../components/controls";
import { IconSessions } from "../../components/icons";
import { ShellOnlyChip } from "../../components/ShellOnly";
import { MEMBER_KINDS, type MemberKind } from "../../lib/ipc/pool-model";
import { DesktopListNote, DesktopReachTag } from "../../pages/pool/connector/DesktopReach";
import { useApps } from "../../lib/apps";
import { EVERYONE, nextRole, REACH_WORDS, type Candidate, type Draft, type DraftMember } from "../../lib/new-pool";
import { MACHINE_WORD, MACHINES_WORD } from "../../lib/machine-words";
import { templateById } from "../../lib/pool-templates";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { NewPoolWhoApps } from "./NewPoolWhoApps";
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

/** What adding one of a kind brings, in a line. */
const KIND_LINES: Record<MemberKind, string> = {
  session: "One session, by itself. Sessions open now come first.",
  app: "An App: every session it starts in these projects joins by itself, now and later.",
  account: "An Account: every session of every App signed in to it joins.",
  tool: "A tool: every session of it joins, on any App or Account.",
  mac: `A ${MACHINE_WORD}: every session on it joins.`,
  everyone: "Everyone: every session in these projects joins, now and later.",
};

/** The rarer kinds, under More (everyone is the switch above). */
const MORE_KINDS: readonly MemberKind[] = ["account", "tool", "mac"];
const isMore = (kind: MemberKind) => MORE_KINDS.includes(kind);

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
}): JSX.Element {
  const [kind, setKind] = useState<MemberKind>(() => {
    const first = templateById(draft.template).kinds[0] ?? "app";
    return MEMBER_KINDS.includes(first) && first !== "everyone" ? first : "app";
  });
  const [query, setQuery] = useState("");
  // Show more's pages so far, by group (none: the first page).
  const [more, setMore] = useState<Partial<Record<Candidate["group"], number>>>({});
  const rows = useMemo(() => {
    const words = query.trim().toLowerCase();
    return candidates[kind].filter((row) => !words || `${row.label} ${row.detail}`.toLowerCase().includes(words));
  }, [candidates, kind, query]);
  const has = (ref: string) => draft.members.some((member) => member.ref === ref);
  const toggle = (row: Candidate) => {
    if (has(row.ref)) onDraft({ ...draft, members: draft.members.filter((member) => member.ref !== row.ref) });
    else {
      const member: DraftMember = { ref: row.ref, kind: row.kind, label: row.label, role: nextRole(draft, row.kind, row.reach), level: null, reach: row.reach, shell: row.shell ?? null };
      onDraft({ ...draft, members: [...draft.members, member] });
    }
  };
  const pickKind = (value: MemberKind) => {
    setKind(value);
    setQuery("");
    setMore({});
  };
  const apps = useApps();
  const everyone = draft.members.find((member) => member.ref === "all");
  const everyoneName = nameOf("all", EVERYONE.label);
  const setEveryone = (on: boolean) =>
    onDraft({ ...draft, members: on ? [...draft.members, { ...EVERYONE, role: templateById(draft.template).broadRole ?? "member" }] : draft.members.filter((member) => member.ref !== "all") });
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
      <section className="flex min-w-0 flex-col gap-3" aria-label="Who's in">
        <span className="flex flex-wrap items-center gap-2">
          <Segmented<MemberKind | "more">
            wrap
            label="Kind of member"
            value={isMore(kind) ? "more" : kind}
            segments={[
              { value: "app", label: `${KIND_WORDS.app} ${candidates.app.length}` },
              { value: "session", label: `${KIND_WORDS.session} ${candidates.session.length}` },
              { value: "more", label: "More", title: "Accounts, tools or Macs" },
            ]}
            onChange={(value) => pickKind(value === "more" ? (isMore(kind) ? kind : "account") : value)}
            testId="new-pool-kind"
            className="self-start"
          />
          {isMore(kind) && (
            <Picker
              choice
              size="sm"
              ariaLabel="Which kind"
              heading="Add a wider group"
              prefix="Kind:"
              value={kind}
              options={MORE_KINDS.map((one) => ({ value: one, label: `${KIND_WORDS[one]} ${candidates[one].length}`, hint: KIND_LINES[one] }))}
              onChange={pickKind}
              testId="new-pool-kind-picker"
            />
          )}
        </span>
        <p className="text-meta text-ink-2" data-testid="new-pool-kind-line">
          {KIND_LINES[kind]}
        </p>
        {kind === "app" ? (
          <NewPoolWhoApps rows={rows} draft={draft} apps={apps ?? []} sessions={sessions} where={where} everyone={everyoneName.charAt(0).toLowerCase() + everyoneName.slice(1)} sessionRows={candidates.session} onToggle={toggle} />
        ) : (
          <CandidateList rows={rows} all={candidates[kind]} kind={kind} query={query} more={more} has={has} onQuery={(words) => {
              setQuery(words);
              setMore({});
            }} onMore={(group) => setMore((was) => ({ ...was, [group]: (was[group] ?? 0) + 1 }))} onToggle={toggle} onFix={onFix} />
        )}
      </section>
    </div>
  );
}

/** Sessions, Accounts, Tools and Macs: a list by group, searched past 8,
 * each group paged. */
function CandidateList({
  rows,
  all,
  kind,
  query,
  more,
  has,
  onQuery,
  onMore,
  onToggle,
  onFix,
}: {
  rows: readonly Candidate[];
  all: readonly Candidate[];
  kind: MemberKind;
  query: string;
  more: Partial<Record<Candidate["group"], number>>;
  has: (ref: string) => boolean;
  onQuery: (words: string) => void;
  onMore: (group: Candidate["group"]) => void;
  onToggle: (row: Candidate) => void;
  onFix: (row: Candidate) => void;
}): JSX.Element {
  return (
    <>
      {all.length > SEARCH_PAST && (
        <input
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          type="search"
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          placeholder={`Search ${KIND_WORDS[kind].toLowerCase()}`}
          aria-label={`Search ${KIND_WORDS[kind].toLowerCase()}`}
          className="h-9 rounded-[10px] bg-fill px-3 text-meta text-ink outline-none inset-ring-1 inset-ring-hairline/75 placeholder:text-muted focus-visible:inset-ring-[1.5px] focus-visible:inset-ring-accent focus-visible:ring-4 focus-visible:ring-accent-soft"
        />
      )}
      {/* Round 7 #9: how a desktop session takes part, once for the list. */}
      <DesktopListNote notes={rows.map((row) => (row.blocked ? undefined : row.note))} testId="new-pool-desktop-note" />
      <div className="flex flex-col gap-4" data-testid="new-pool-candidates">
        {GROUPS.map((group) => {
          const inGroup = rows.filter((row) => row.group === group.id);
          if (inGroup.length === 0) return null;
          const first = group.id === "all" && !query ? SEARCH_PAST : CANDIDATES_PAGE;
          const cut = inGroup.slice(0, first + (more[group.id] ?? 0) * CANDIDATES_PAGE);
          return (
            <div key={group.id} className="flex flex-col gap-1.5">
              <span className="flex items-baseline gap-2.5">
                <span className="section-label">{group.label}</span>
                <span className="text-label text-faint">{inGroup.length}</span>
              </span>
              <ul className="flex flex-col gap-0.5 rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
                {cut.map((row) => (
                  <CandidateRow key={row.ref} row={row} on={has(row.ref)} onToggle={() => onToggle(row)} onFix={() => onFix(row)} />
                ))}
              </ul>
              {cut.length < inGroup.length && (
                <Button variant="ghost" size="sm" className="self-start" onClick={() => onMore(group.id)} data-testid={`new-pool-more-${group.id}`}>
                  Show more ({inGroup.length - cut.length} more)
                </Button>
              )}
            </div>
          );
        })}
        {rows.length === 0 && <p className="text-meta text-ink-2">{query ? "Nothing matches." : `No ${KIND_WORDS[kind].toLowerCase()} yet.`}</p>}
      </div>
    </>
  );
}

function CandidateRow({ row, on, onToggle, onFix }: { row: Candidate; on: boolean; onToggle: () => void; onFix: () => void }): JSX.Element {
  const blocked = row.blocked !== null;
  return (
    <li className={"flex flex-col gap-1 rounded-[10px] px-3 py-2 " + (on ? "bg-accent-soft" : blocked ? "" : "hover:bg-surface-2")} data-testid="new-pool-candidate" data-ref={row.ref}>
      <button
        type="button"
        role="checkbox"
        aria-checked={on}
        disabled={blocked}
        onClick={onToggle}
        className="flex min-w-0 items-center gap-3 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-default"
      >
        <span aria-hidden="true" className={"flex size-5 shrink-0 items-center justify-center rounded-md text-label " + (on ? "bg-accent-fill text-white" : blocked ? "inset-ring-[1.5px] inset-ring-hairline" : "inset-ring-[1.5px] inset-ring-faint")}>
          {on ? "✓" : ""}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className={"truncate text-body font-medium " + (blocked ? "text-ink-2" : "text-ink")}>{row.label}</span>
          <span className="truncate text-meta text-ink-2">{row.detail}</span>
        </span>
        {/* A row that can't join has no reach to promise (the P6 walk: a
            paired Mac said "new ones join" beside why it can't). */}
        {!blocked && <span className="shrink-0 text-meta text-muted">{REACH_WORDS[row.reach]}</span>}
        {/* Round 7 #9: where its app stands, in a word (the list says why). */}
        {!blocked && row.note && !on && <DesktopReachTag member={row.ref} testId="new-pool-candidate-reach" />}
        {!blocked && row.shell && <ShellOnlyChip why={row.shell} inRow />}
      </button>
      {(row.blocked || (row.fix && row.reach === "watched")) && (
        <span className="flex flex-wrap items-center gap-2 pl-8 text-meta text-ink-2">
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
