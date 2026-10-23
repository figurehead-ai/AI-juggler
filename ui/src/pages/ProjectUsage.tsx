/**
 * AJ-E1533 — a project's Usage: what its sessions used, by account and
 * tool, from the same numbers each session's own page shows (AI
 * Juggler's sessions, and the ones it didn't start; an AI Juggler
 * session's own conversation is counted once, as the session).
 */
import { AccountAvatar } from "../components/Avatar";
import type { JSX } from "react";
import { Card } from "../components/primitives";
import { ToolMark } from "../components/ToolMark";
import { outside, useConversations } from "../lib/conversations";
import type { InstanceInfo } from "../lib/bridge";

type Tokens = { input: number; output: number; cache_read: number; cache_write: number };

type Line = { accountId: string | null; label: string; tool: string; sessions: number; io: number; cache: number };

const count = (n: number) => (n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1000 ? `${Math.round(n / 1000)}k` : `${n}`);

export function ProjectUsage({ projectId, sessions }: { projectId: string; sessions: InstanceInfo[] }): JSX.Element {
  const { all } = useConversations();
  const lines = new Map<string, Line>();
  const add = (accountId: string | null, label: string, tool: string, tokens: Tokens | null) => {
    const line = lines.get(accountId ?? label) ?? { accountId, label, tool, sessions: 0, io: 0, cache: 0 };
    line.sessions += 1;
    if (tokens) {
      line.io += tokens.input + tokens.output;
      line.cache += tokens.cache_read + tokens.cache_write;
    }
    lines.set(accountId ?? label, line);
  };
  for (const s of sessions.filter((row) => row.project_id === projectId)) add(s.account_id, s.account_label, s.provider, s.tokens);
  for (const c of outside(all).filter((row) => row.project_id === projectId)) add(c.account_id, c.account_label, c.tool, c.tokens);
  const rows = [...lines.values()].sort((a, b) => b.io - a.io);
  const total = rows.reduce((sum, row) => sum + row.io, 0);
  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="project-usage">
      <p className="text-[11px] uppercase tracking-wider text-muted">By account</p>
      <table className="w-full text-[12px]">
        <thead>
          <tr className="text-left text-[11px] text-muted">
            <th className="py-1 font-normal">Account</th>
            <th className="py-1 text-right font-normal">Sessions</th>
            <th className="py-1 text-right font-normal">Input + output</th>
            <th className="py-1 text-right font-normal">Cache</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.accountId ?? row.label} className="border-t border-hairline/60" data-testid="project-usage-row">
              <td className="py-1.5">
                <span className="flex items-center gap-2">
                  <AccountAvatar accountId={row.accountId} label={row.label} size={16} />
                  <ToolMark tool={row.tool} size={13} className="text-muted" />
                  <span className="text-ink">{row.label}</span>
                </span>
              </td>
              <td className="py-1.5 text-right tabular-nums text-muted">{row.sessions}</td>
              <td className="py-1.5 text-right tabular-nums text-ink">{count(row.io)}</td>
              <td className="py-1.5 text-right tabular-nums text-muted">{count(row.cache)}</td>
            </tr>
          ))}
          <tr className="border-t border-hairline">
            <td className="py-1.5 text-muted">All accounts</td>
            <td className="py-1.5 text-right tabular-nums text-muted">{rows.reduce((sum, row) => sum + row.sessions, 0)}</td>
            <td className="py-1.5 text-right font-medium tabular-nums text-ink" data-testid="project-usage-total">
              {count(total)}
            </td>
            <td />
          </tr>
        </tbody>
      </table>
      <p className="text-[10px] text-muted">Measured: read from each session's own files, each message counted once.</p>
    </Card>
  );
}
