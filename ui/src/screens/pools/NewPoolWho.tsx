/**
 * AJ-E3007 step 2, AJ-E3011's "Who's in": who works together, picked from
 * what exists (wireframe 2). Where is its own step before it.
 *
 * - By kind: Apps (every session they start here joins) and sessions open
 *   now first, then Accounts, Tools, Macs and everyone; each kind says in
 *   a line what adding one brings. Open now first, then Recent, then All;
 *   search past 8. A row that can't join says why and offers the step
 *   that would let it in.
 * - The side panel counts what each choice brings, what joins later, and
 *   what each member can do here.
 * - Calm (D-117): surface tone, no row dividers; colour only for what
 *   needs you.
 */
import { useMemo, useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { Segmented } from "../../components/controls";
import { ShellOnlyChip } from "../../components/ShellOnly";
import { MEMBER_KINDS, type MemberKind } from "../../lib/ipc/pool-model";
import { nextRole, REACH_WORDS, type Candidate, type Draft, type DraftMember, type Reach } from "../../lib/new-pool";
import { MACHINE_WORD, MACHINES_WORD } from "../../lib/machine-words";
import { templateById } from "../../lib/pool-templates";
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

/** The kinds in the order a newcomer reaches for them. */
const KIND_ORDER: readonly MemberKind[] = ["app", "session", "account", "tool", "mac", "everyone"];

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
}: {
  draft: Draft;
  onDraft: (next: Draft) => void;
  candidates: Record<MemberKind, Candidate[]>;
  onFix: (candidate: Candidate) => void;
  /** A member's name on screen (everyone by its scope). */
  nameOf?: (ref: string, label: string) => string;
}): JSX.Element {
  const [kind, setKind] = useState<MemberKind>(() => {
    const first = templateById(draft.template).kinds[0] ?? "session";
    return MEMBER_KINDS.includes(first) ? first : "session";
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
  const broad = draft.members.filter((member) => member.reach === "later");
  // An App or Tool whose sessions all work through their shell counts
  // there too (the P8 walk: a Kimi Code App left "0 shell only").
  const count = (reach: Reach) => draft.members.filter((member) => member.reach === reach || (reach === "shell" && member.reach === "later" && member.shell)).length;
  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1.6fr)_minmax(240px,1fr)]">
      <div className="flex min-w-0 flex-col gap-4">
        <StepHead step="who" />
        <section className="flex flex-col gap-2" aria-label="Who's in">
          <span className="text-meta font-medium text-ink" id="new-pool-kind-label">
            Add Apps, sessions open now, or a wider group
          </span>
          <Segmented
            calm
            wrap
            label="Kind of member"
            value={kind}
            segments={KIND_ORDER.map((one) => ({ value: one, label: one === "everyone" ? KIND_WORDS[one] : `${KIND_WORDS[one]} ${candidates[one].length}` }))}
            onChange={(value) => {
              setKind(value);
              setQuery("");
              setMore({});
            }}
            testId="new-pool-kind"
            className="self-start"
          />
          <p className="text-meta text-ink-2" data-testid="new-pool-kind-line">
            {KIND_LINES[kind]}
          </p>
          {candidates[kind].length > SEARCH_PAST && (
            <input autoCorrect="off" autoCapitalize="off" spellCheck={false}
              type="search"
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setMore({});
              }}
              placeholder={`Search ${KIND_WORDS[kind].toLowerCase()}`}
              aria-label={`Search ${KIND_WORDS[kind].toLowerCase()}`}
              className="seg-fill h-8 rounded-lg bg-fill px-2.5 text-meta text-ink outline-none placeholder:text-muted focus-visible:ring-2 focus-visible:ring-accent/60"
            />
          )}
          <div className="flex flex-col gap-3" data-testid="new-pool-candidates">
            {GROUPS.map((group) => {
              const inGroup = rows.filter((row) => row.group === group.id);
              if (inGroup.length === 0) return null;
              const first = group.id === "all" && !query ? SEARCH_PAST : CANDIDATES_PAGE;
              const cut = inGroup.slice(0, first + (more[group.id] ?? 0) * CANDIDATES_PAGE);
              return (
                <div key={group.id} className="flex flex-col">
                  <span className="section-label mb-1">
                    {group.label} <span className="text-muted">{inGroup.length}</span>
                  </span>
                  <ul className="flex flex-col gap-0.5 rounded-xl bg-surface p-1">
                    {cut.map((row) => (
                      <CandidateRow key={row.ref} row={row} on={has(row.ref)} onToggle={() => toggle(row)} onFix={() => onFix(row)} />
                    ))}
                  </ul>
                  {cut.length < inGroup.length && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="self-start"
                      onClick={() => setMore((was) => ({ ...was, [group.id]: (was[group.id] ?? 0) + 1 }))}
                      data-testid={`new-pool-more-${group.id}`}
                    >
                      Show more ({inGroup.length - cut.length} more)
                    </Button>
                  )}
                </div>
              );
            })}
            {rows.length === 0 && <p className="text-meta text-ink-2">{query ? "Nothing matches." : `No ${KIND_WORDS[kind].toLowerCase()} yet.`}</p>}
          </div>
        </section>
      </div>

      <aside className="flex flex-col gap-3 self-start rounded-xl bg-surface-2 p-3" aria-label="In this pool" data-testid="new-pool-side">
        <span className="text-body font-semibold text-ink">
          In this pool <span className="text-meta font-medium text-muted">{draft.members.length}</span>
        </span>
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-meta">
          <dt className="text-muted">Members</dt>
          <dd className="text-ink">{draft.members.length > 0 ? draft.members.map((member) => nameOf(member.ref, member.label)).join(", ") : "none yet"}</dd>
          <dt className="text-muted">Joins later</dt>
          <dd className="text-ink">
            {broad.some((member) => member.ref === "all")
              ? "every new session in these projects"
              : broad.length > 0
                ? `new sessions of ${broad.map((member) => nameOf(member.ref, member.label)).join(", ")}`
                : `nothing (no App, Account, Tool or ${MACHINE_WORD} chosen)`}
          </dd>
        </dl>
        <div className="flex flex-col gap-1">
          <span className="section-label">Can act in the pool</span>
          <span className="flex flex-wrap gap-x-3 gap-y-1 text-meta text-ink-2">
            <span>{count("full")} full</span>
            <span>{count("shell")} shell only</span>
            <span>{count("watched")} watched</span>
          </span>
        </div>
      </aside>
    </div>
  );
}

function CandidateRow({ row, on, onToggle, onFix }: { row: Candidate; on: boolean; onToggle: () => void; onFix: () => void }): JSX.Element {
  const blocked = row.blocked !== null;
  return (
    <li className={"flex flex-col gap-1 rounded-lg px-3 py-2 " + (on ? "bg-surface-3" : blocked ? "" : "hover:bg-surface-2")} data-testid="new-pool-candidate" data-ref={row.ref}>
      <button
        type="button"
        role="checkbox"
        aria-checked={on}
        disabled={blocked}
        onClick={onToggle}
        className="flex min-w-0 items-center gap-2.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-default"
      >
        <span aria-hidden="true" className={"flex size-4 shrink-0 items-center justify-center rounded border text-label " + (on ? "border-accent bg-accent text-white" : blocked ? "border-muted/40" : "border-muted")}>
          {on ? "✓" : ""}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className={"truncate text-body font-medium " + (blocked ? "text-ink-2" : "text-ink")}>{row.label}</span>
          <span className="truncate text-meta text-ink-2">{row.detail}</span>
        </span>
        {/* A row that can't join has no reach to promise (the P6 walk: a
            paired Mac said "new ones join" beside why it can't). */}
        {!blocked && <span className="shrink-0 text-meta text-muted">{REACH_WORDS[row.reach]}</span>}
        {!blocked && row.shell && <ShellOnlyChip why={row.shell} inRow />}
      </button>
      {!row.blocked && row.note && (
        <span className="pl-6 text-meta text-ink-2" data-testid="new-pool-candidate-note">
          {row.note}
        </span>
      )}
      {(row.blocked || (row.fix && row.reach === "watched")) && (
        <span className="flex flex-wrap items-center gap-2 pl-6 text-meta text-ink-2">
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
