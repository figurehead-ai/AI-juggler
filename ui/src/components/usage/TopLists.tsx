/**
 * 0.1.5 — the projects and sessions that used the most: the top ten as a
 * table (names, never ids; compact numbers with the exact one on hover;
 * a share bar split by profile), and "See all", a list read a page at a
 * time. Each row opens its page.
 */
import { useState, type JSX } from "react";
import { AccountAvatar } from "../Avatar";
import { Modal } from "../form";
import { PagedList } from "../PagedList";
import { useActionHost } from "../ItemMenu";
import { useConversations } from "../../lib/conversations";
import { usePagedList } from "../../lib/paged";
import { usageHistorySessions, type UsageHistoryInfo, type UsageSessionInfo, type UsageShareInfo } from "../../lib/ipc/usageHistory";
import { RANGES, type RangeId } from "../../lib/usage/range";
import { accountColor } from "../../lib/palette";
import { Section } from "../charts/ChartCard";
import { ShareBar } from "../charts/StatTile";
import { exact, short, usdWords } from "../charts/scale";
import { IconFolder, IconList } from "./controls";
import type { Scope } from "./useUsage";

const TOP = 10;

/** When, in one short line: "12 min ago", "5 h ago", "2 days ago", then "Oct 23". */
export function whenWords(ms: number, now: number = Date.now()): string {
  const minutes = Math.max(0, Math.round((now - ms) / 60_000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return days === 1 ? "yesterday" : `${days} days ago`;
  return new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function SeeAll({ count, noun, onOpen }: { count: number; noun: string; onOpen: () => void }): JSX.Element | null {
  if (count <= TOP) return null;
  return (
    <button
      type="button"
      onClick={onOpen}
      className="h-8 rounded-lg border border-hairline bg-surface-2 px-2.5 text-label font-medium text-ink outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid="see-all"
    >
      See all {count} {noun}
    </button>
  );
}

const TH = "sticky top-0 bg-surface px-2.5 pb-2 text-meta font-semibold text-muted";

function profileParts(share: UsageShareInfo, labels: Map<string, string>) {
  return share.profiles.map((part) => ({ key: part.profile, label: labels.get(part.profile) ?? "A profile", value: part.io, color: accountColor(part.profile) }));
}

export function TopProjects({ data, busy, className = "" }: { data: UsageHistoryInfo | null; busy: boolean; className?: string }): JSX.Element {
  const host = useActionHost();
  const [all, setAll] = useState(false);
  const labels = new Map((data?.by_profile ?? []).map((share) => [share.key, share.label]));
  const total = data ? data.totals.tokens.input + data.totals.tokens.output : 0;
  const rows = (data?.by_project ?? []).filter((share) => share.io > 0);
  const row = (share: UsageShareInfo) => (
    <tr key={share.key} className="group border-t border-hairline/60 hover:bg-surface-2" data-testid="top-project">
      <td className="px-2.5 py-2">
        {share.key ? (
          <button type="button" className="text-left text-body font-semibold text-ink outline-none hover:text-accent-hi focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => host.openProjectById?.(share.key)}>
            {share.label}
          </button>
        ) : (
          <span className="text-body font-semibold text-muted">{share.label}</span>
        )}
        <span className="block truncate text-meta text-muted">
          {share.sessions} {share.sessions === 1 ? "session" : "sessions"} · {share.profiles.map((part) => labels.get(part.profile) ?? "A profile").join(", ")}
        </span>
      </td>
      <td className="px-2.5 py-2 text-right text-body tabular-nums" title={exact(share.io, "tokens")}>
        {short(share.io, "tokens")}
      </td>
      <td className="px-2.5 py-2 text-right text-body tabular-nums" title={share.usd == null ? "No list price" : exact(share.usd, "usd")}>
        {share.usd == null ? "—" : usdWords(share.usd)}
      </td>
      <td className="w-[34%] px-2.5 py-2">
        <ShareBar parts={profileParts(share, labels)} total={total} scale={rows[0]?.io} unit="tokens" label={share.label} />
      </td>
    </tr>
  );
  const table = (shown: UsageShareInfo[]) => (
    <table className="w-full border-collapse">
      <thead>
        <tr>
          <th scope="col" className={TH + " text-left"}>Project</th>
          <th scope="col" className={TH + " text-right"}>Tokens</th>
          <th scope="col" className={TH + " text-right"}>Cost</th>
          <th scope="col" className={TH + " text-left"}>Share</th>
        </tr>
      </thead>
      <tbody>{shown.map(row)}</tbody>
    </table>
  );
  return (
    <Section title="Top projects" icon={<IconFolder />} sub="by tokens in and out" className={className} busy={busy} testId="top-projects" actions={<SeeAll count={rows.length} noun="projects" onOpen={() => setAll(true)} />}>
      {rows.length === 0 ? <p className="py-6 text-center text-meta text-muted">{data ? "No project used anything in this range." : "Reading usage…"}</p> : <div className="overflow-x-auto">{table(rows.slice(0, TOP))}</div>}
      {all && (
        <Modal title="Every project, by usage" subtitle={`${rows.length} projects`} wide onClose={() => setAll(false)} testId="all-projects">
          <div className="max-h-[60vh] overflow-y-auto">{table(rows)}</div>
        </Modal>
      )}
    </Section>
  );
}

/** A session's name: its title, else "Untitled session · its project". */
export function sessionName(row: UsageSessionInfo, projects: Map<string, string>): string {
  if (row.title?.trim()) return row.title.trim();
  const project = row.project_id ? projects.get(row.project_id) : null;
  return project ? `Untitled session · ${project}` : "Untitled session";
}

function SessionRow({ row, projects, labels, total }: { row: UsageSessionInfo; projects: Map<string, string>; labels: Map<string, string>; total: number }): JSX.Element {
  const host = useActionHost();
  const { all } = useConversations();
  const conversation = row.session_id ? null : all.find((entry) => entry.account_id === row.profile && entry.id === row.id);
  const open = row.session_id ? () => host.openSessionById?.(row.session_id!) : conversation ? () => host.openConversation?.(conversation) : null;
  const name = sessionName(row, projects);
  const profile = labels.get(row.profile) ?? "A profile";
  const where = [row.project_id ? projects.get(row.project_id) : null, whenWords(row.last_ms)].filter(Boolean).join(" · ");
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,9rem)_5rem_4.5rem] items-center gap-3 px-2.5 py-2 hover:bg-surface-2" data-testid="top-session">
      <span className="min-w-0">
        {open ? (
          <button type="button" onClick={open} className="block max-w-full truncate text-left text-body font-semibold text-ink outline-none hover:text-accent-hi focus-visible:ring-2 focus-visible:ring-accent/60" title={name}>
            {name}
          </button>
        ) : (
          <span className="block truncate text-body font-semibold text-ink" title={name}>
            {name}
          </span>
        )}
        <span className="block truncate text-meta text-muted">{where}</span>
      </span>
      <span className="flex min-w-0 items-center gap-2 text-meta text-ink">
        <AccountAvatar accountId={row.profile} label={profile} size={20} />
        <span className="truncate">
          <span className="sr-only">on </span>
          {profile}
        </span>
      </span>
      <span className="text-right text-body tabular-nums" title={`${exact(row.io, "tokens")}${total > 0 ? `, ${((row.io / total) * 100).toFixed(1)}% of the range` : ""}`}>
        {short(row.io, "tokens")}
        <span className="sr-only"> tokens</span>
      </span>
      <span className="text-right text-body tabular-nums" title={row.usd == null ? "No list price" : exact(row.usd, "usd")}>
        {row.usd == null ? "—" : usdWords(row.usd)}
        <span className="sr-only"> estimated</span>
      </span>
    </div>
  );
}

function Header(): JSX.Element {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,9rem)_5rem_4.5rem] gap-3 border-b border-hairline px-2.5 pb-2 text-meta font-semibold text-muted" aria-hidden="true">
      <span>Session</span>
      <span>Ran on</span>
      <span className="text-right">Tokens</span>
      <span className="text-right">Cost</span>
    </div>
  );
}

export function TopSessions({ data, busy, range, scope, className = "" }: { data: UsageHistoryInfo | null; busy: boolean; range: RangeId; scope: Scope; className?: string }): JSX.Element {
  const [all, setAll] = useState(false);
  const projects = new Map((data?.by_project ?? []).map((share) => [share.key, share.label]));
  const labels = new Map((data?.by_profile ?? []).map((share) => [share.key, share.label]));
  const total = data ? data.totals.tokens.input + data.totals.tokens.output : 0;
  const rows = data?.top_sessions ?? [];
  return (
    <Section
      title="Top sessions"
      icon={<IconList />}
      sub="by tokens in and out"
      className={className}
      busy={busy}
      testId="top-sessions"
      actions={<SeeAll count={data?.session_count ?? 0} noun="sessions" onOpen={() => setAll(true)} />}
    >
      {rows.length === 0 ? (
        <p className="py-6 text-center text-meta text-muted">{data ? "No session used anything in this range." : "Reading usage…"}</p>
      ) : (
        <div>
          <Header />
          <ul aria-label="Top sessions" className="divide-y divide-hairline/60">
            {rows.slice(0, TOP).map((row) => (
              <li key={`${row.profile}:${row.id}`}>
                <SessionRow row={row} projects={projects} labels={labels} total={total} />
              </li>
            ))}
          </ul>
        </div>
      )}
      {all && data && <AllSessions data={data} range={range} scope={scope} projects={projects} labels={labels} total={total} onClose={() => setAll(false)} />}
    </Section>
  );
}

function AllSessions({ data, range, scope, projects, labels, total, onClose }: { data: UsageHistoryInfo; range: RangeId; scope: Scope; projects: Map<string, string>; labels: Map<string, string>; total: number; onClose: () => void }): JSX.Element {
  const edges = data.edges_ms;
  const input = { edges_ms: edges, group_by: data.group_by, profile: scope.profile ?? null, project: scope.project ?? null, top: 0 };
  const key = `usage-sessions:${JSON.stringify([edges[0], edges[edges.length - 1], input.profile, input.project])}`;
  const list = usePagedList(key, (request) => usageHistorySessions(input, request), { pageSize: 50 });
  const spoken = RANGES.find((entry) => entry.id === range)?.spoken ?? "";
  return (
    <Modal title="Every session, by usage" subtitle={`${data.session_count} sessions in ${spoken}`} wide onClose={onClose} testId="all-sessions">
      <Header />
      <PagedList
        list={list}
        keyOf={(row) => `${row.profile}:${row.id}`}
        renderRow={(row) => <SessionRow row={row} projects={projects} labels={labels} total={total} />}
        rowHeight={56}
        label="Every session, by usage"
        empty={<p className="py-6 text-center text-meta text-muted">No session used anything in this range.</p>}
        maxHeight={520}
      />
    </Modal>
  );
}

