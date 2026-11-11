/**
 * AJ-E3007 step 2: where and who, picked from what exists (wireframe 2).
 *
 * - Where: every project, or the ones picked (chips, "Add a project").
 * - Who: by kind (sessions, Apps, Accounts, Tools, Macs, everyone), Open
 *   now first, then Recent, then All; search past 8. A row that can't join
 *   says why and offers the step that would let it in.
 * - The side panel counts what each choice brings, what joins later, and
 *   what each member can do here.
 */
import { projectChoices } from "../../lib/project-words";
import { useMemo, useState, type JSX } from "react";

import { Button, Chip } from "../../components/primitives";
import { Picker, Segmented } from "../../components/controls";
import { ShellOnlyChip } from "../../components/ShellOnly";
import { MEMBER_KINDS, type MemberKind } from "../../lib/ipc/pool-model";
import { nextRole, REACH_WORDS, type Candidate, type Draft, type DraftMember, type Reach } from "../../lib/new-pool";
import { MACHINE_WORD, MACHINES_WORD } from "../../lib/machine-words";

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

const GROUPS: { id: Candidate["group"]; label: string }[] = [
  { id: "open", label: "Open now" },
  { id: "recent", label: "Recent" },
  { id: "all", label: "All" },
];

export function NewPoolWho({
  draft,
  onDraft,
  candidates,
  projects,
  onFix,
}: {
  draft: Draft;
  onDraft: (next: Draft) => void;
  candidates: Record<MemberKind, Candidate[]>;
  /** Every project; `open`: it has sessions open now (shown first). */
  projects: { id: string; name: string; root?: string; open: boolean }[];
  onFix: (candidate: Candidate) => void;
}): JSX.Element {
  const [kind, setKind] = useState<MemberKind>("session");
  const [query, setQuery] = useState("");
  // Show more's pages so far, by group (none: the first page).
  const [more, setMore] = useState<Partial<Record<Candidate["group"], number>>>({});
  const name = (id: string | null) => (id === null ? "No project" : (projects.find((project) => project.id === id)?.name ?? "A project"));
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
  const addable = projects.filter((project) => !draft.projects.includes(project.id));
  const broad = draft.members.filter((member) => member.reach === "later");
  // An App or Tool whose sessions all work through their shell counts
  // there too (the P8 walk: a Kimi Code App left "0 shell only").
  const count = (reach: Reach) => draft.members.filter((member) => member.reach === reach || (reach === "shell" && member.reach === "later" && member.shell)).length;
  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1.6fr)_minmax(240px,1fr)]">
      <div className="flex min-w-0 flex-col gap-4">
        <section className="flex flex-col gap-2 rounded-card border border-hairline bg-surface p-3" aria-label="Where">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-label font-semibold uppercase tracking-wide text-muted">Where</span>
            <Segmented
              label="Where it works"
              value={draft.every ? "every" : "these"}
              segments={[
                { value: "every", label: "Every project" },
                { value: "these", label: "These projects" },
              ]}
              onChange={(value) => onDraft({ ...draft, every: value === "every" })}
              testId="new-pool-where"
            />
          </div>
          {!draft.every && (
            <div className="flex flex-wrap items-center gap-1.5" data-testid="new-pool-projects">
              {draft.projects.map((id) => (
                <span key={id ?? "none"} className="inline-flex items-center gap-1 rounded-full border border-hairline bg-surface-2 py-0.5 pl-2.5 pr-1 text-meta text-ink">
                  {name(id)}
                  <button type="button" aria-label={`Take ${name(id)} out`} className="rounded-full px-1 text-muted hover:text-ink" onClick={() => onDraft({ ...draft, projects: draft.projects.filter((one) => one !== id) })}>
                    ×
                  </button>
                </span>
              ))}
              <Picker
                ariaLabel="Add a project"
                heading="Add a project to the scope"
                value={null}
                groups={[{ label: "Open now", values: addable.filter((project) => project.open).map((project) => project.id) }]}
                allHeading="All projects"
                searchLabel="Search projects"
                size="sm"
                placeholder="+ Add a project"
                options={[...projectChoices(addable), ...(draft.projects.includes(null) ? [] : [{ value: "__none", label: "Sessions with no project" }])]}
                onChange={(value) => onDraft({ ...draft, projects: [...draft.projects, value === "__none" ? null : value] })}
                testId="new-pool-add-project"
              />
            </div>
          )}
          <p className="text-meta text-ink-2">{draft.every ? "Every project, and sessions with no project. Agents see only this pool's members." : "Agents see only this pool's projects."}</p>
        </section>

        <section className="flex flex-col gap-2" aria-label="Who">
          <span className="text-label font-semibold uppercase tracking-wide text-muted">Who · pick sessions, or whole Apps, Accounts, Tools or {MACHINES_WORD}</span>
          <Segmented
            label="Kind of member"
            value={kind}
            segments={MEMBER_KINDS.map((one) => ({ value: one, label: one === "everyone" ? KIND_WORDS[one] : `${KIND_WORDS[one]} ${candidates[one].length}` }))}
            onChange={(value) => {
              setKind(value);
              setQuery("");
              setMore({});
            }}
            testId="new-pool-kind"
          />
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
              className="h-8 rounded-lg border border-hairline bg-surface-2 px-2.5 text-meta text-ink outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
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
                  <span className="mb-1 text-label font-semibold uppercase tracking-wide text-accent-hi">
                    {group.label} <span className="text-muted">{inGroup.length}</span>
                  </span>
                  <ul className="flex flex-col divide-y divide-hairline/60 rounded-card border border-hairline bg-surface">
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

      <aside className="flex flex-col gap-3 self-start rounded-card border border-hairline bg-surface-2 p-3" aria-label="In this pool" data-testid="new-pool-side">
        <span className="text-body font-semibold text-ink">
          In this pool <span className="text-meta font-medium text-accent-hi">{draft.members.length}</span>
        </span>
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-meta">
          <dt className="text-muted">Projects</dt>
          <dd className="text-ink">{draft.every ? "Every project" : draft.projects.length > 0 ? draft.projects.map(name).join(", ") : "none yet"}</dd>
          <dt className="text-muted">Members</dt>
          <dd className="text-ink">{draft.members.length > 0 ? draft.members.map((member) => member.label).join(", ") : "none yet"}</dd>
          <dt className="text-muted">Joins later</dt>
          <dd className="text-ink">{broad.length > 0 ? `new sessions of ${broad.map((member) => member.label).join(", ")}` : `nothing (no App, Account, Tool or ${MACHINE_WORD} chosen)`}</dd>
        </dl>
        <div className="flex flex-col gap-1">
          <span className="text-label font-semibold uppercase tracking-wide text-muted">Can act in the pool</span>
          <span className="flex flex-wrap gap-1.5">
            <Chip tone="success">{count("full")} full</Chip>
            <Chip tone="neutral">{count("shell")} shell only</Chip>
            <Chip tone="warning">{count("watched")} watched</Chip>
          </span>
        </div>
      </aside>
    </div>
  );
}

function CandidateRow({ row, on, onToggle, onFix }: { row: Candidate; on: boolean; onToggle: () => void; onFix: () => void }): JSX.Element {
  const blocked = row.blocked !== null;
  return (
    <li className={"flex flex-col gap-1 px-3 py-2 " + (blocked ? "bg-surface-2" : "")} data-testid="new-pool-candidate" data-ref={row.ref}>
      <button
        type="button"
        role="checkbox"
        aria-checked={on}
        disabled={blocked}
        onClick={onToggle}
        className="flex min-w-0 items-center gap-2.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-default"
      >
        <span aria-hidden="true" className={"flex size-4 shrink-0 items-center justify-center rounded border " + (on ? "border-accent bg-accent text-white" : blocked ? "border-dashed border-muted/60" : "border-muted")}>
          {on ? "✓" : ""}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className={"truncate text-body font-medium " + (blocked ? "text-ink-2" : "text-ink")}>{row.label}</span>
          <span className="truncate text-meta text-ink-2">{row.detail}</span>
        </span>
        {/* A row that can't join has no reach to promise (the P6 walk: a
            paired Mac said "new ones join" beside why it can't). */}
        {!blocked && <Chip tone={row.reach === "full" ? "success" : row.reach === "watched" ? "warning" : row.reach === "later" ? "accent" : "neutral"}>{REACH_WORDS[row.reach]}</Chip>}
        {!blocked && row.shell && <ShellOnlyChip why={row.shell} inRow />}
      </button>
      {!row.blocked && row.note && (
        <span className="pl-6 text-meta text-warning" data-testid="new-pool-candidate-note">
          {row.note}
        </span>
      )}
      {(row.blocked || (row.fix && row.reach === "watched")) && (
        <span className="flex flex-wrap items-center gap-2 pl-6 text-meta text-warning">
          {row.blocked ?? "Runs in another app: it can be watched, not act."}
          {row.fix && (
            <Button size="sm" variant="secondary" onClick={onFix} data-testid="new-pool-fix">
              {row.fix.label}
            </Button>
          )}
        </span>
      )}
    </li>
  );
}
