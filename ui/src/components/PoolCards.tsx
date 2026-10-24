import { useState, type JSX } from "react";

import type { InstanceInfo } from "../lib/bridge";
import type { CardField, ContextCard } from "../lib/ipc/pools";
import { roleLabel } from "../lib/pools";
import { agoWords } from "../lib/time-words";
import { stateChip } from "./indicator";
import { useActionHost } from "./ItemMenu";
import { ProjectMark } from "./ProjectMark";
import { ToolMark } from "./ToolMark";

/**
 * AJ-E1703: a pool's context cards, as rows like Home's — name, place,
 * state, tool and last activity — with the rest behind each row's
 * disclosure: its role, what it says it's doing (marked as its own
 * words), its task and the files it changed. A field AI Juggler doesn't
 * know says "Unavailable" and why (D-06).
 */
export function PoolCardRows({ cards, sessions, now }: { cards: ContextCard[]; sessions: InstanceInfo[]; now: number }): JSX.Element {
  if (cards.length === 0) {
    return (
      <p className="text-xs text-muted" data-testid="pool-cards-empty">
        No sessions here yet. Move one in from its menu in the list.
      </p>
    );
  }
  return (
    <ul className="overflow-hidden rounded-xl border border-hairline bg-surface" data-testid="pool-cards">
      {cards.map((card) => (
        <CardRow key={card.session_id} card={card} session={sessions.find((row) => row.id === card.session_id)} now={now} />
      ))}
    </ul>
  );
}

function CardRow({ card, session, now }: { card: ContextCard; session: InstanceInfo | undefined; now: number }): JSX.Element {
  const host = useActionHost();
  const [open, setOpen] = useState(false);
  const state = stateChip(card.state);
  const detailId = `pool-card-${card.session_id}`;
  return (
    <li data-home-item className="border-t border-hairline/60 first:border-t-0" data-testid="pool-card">
      <div className="flex items-center">
        <button
          type="button"
          data-home-row
          className="flex min-w-0 flex-1 items-center gap-3.5 px-4 py-2.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60"
          onClick={() => host.openSessionById?.(card.session_id)}
        >
          <state.Icon aria-hidden="true" width={13} height={13} className="shrink-0 text-muted" />
          <span className="sr-only">{state.label}: </span>
          <ProjectMark projectId={session?.project_id ?? null} size={16} />
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-sm text-ink">{card.name}</span>
            <span className="truncate text-xs text-faint">{card.place}</span>
          </span>
          <span className="flex shrink-0 items-center gap-1.5 text-xs text-muted">
            {session && <ToolMark tool={session.provider} size={12} className="shrink-0" />}
            <span>
              {card.tool}
              {card.account ? ` · ${card.account}` : ""}
            </span>
          </span>
          {card.role === "lead" && <span className="shrink-0 rounded-md bg-accent-soft px-1.5 text-[10.5px] text-accent-hi">Lead</span>}
          <span className="w-20 shrink-0 whitespace-nowrap text-right font-mono text-xs text-faint">
            {card.last_activity_ms === null ? "—" : agoWords(card.last_activity_ms, now)}
          </span>
        </button>
        <button
          type="button"
          aria-expanded={open}
          aria-controls={detailId}
          aria-label={`${open ? "Hide" : "Show"} what ${card.name} is doing`}
          onClick={() => setOpen((shown) => !shown)}
          className="mr-2 flex size-7 shrink-0 items-center justify-center rounded-md text-faint outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="pool-card-more"
        >
          <svg aria-hidden="true" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
            <path d={open ? "M6 15l6-6 6 6" : "M6 9l6 6 6-6"} />
          </svg>
        </button>
      </div>
      {open && (
        <dl id={detailId} className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5 border-t border-hairline/40 px-4 py-2.5 text-[12px]" data-testid="pool-card-detail">
          <dt className="text-muted">Role</dt>
          <dd className="text-ink">{roleLabel(card.role)}</dd>
          <dt className="text-muted">It says</dt>
          <dd data-testid="pool-card-said">
            <Field field={card.said} said />
          </dd>
          <dt className="text-muted">Task</dt>
          <dd>
            <Field field={card.task} />
          </dd>
          <dt className="text-muted">Files changed</dt>
          <dd data-testid="pool-card-files">
            {card.files_unavailable ? (
              <span className="text-faint">{card.files_unavailable}</span>
            ) : card.files.length === 0 ? (
              <span className="text-muted">None yet</span>
            ) : (
              <ul className="flex flex-col gap-0.5 font-mono text-[11.5px] text-ink">
                {card.files.map((file) => (
                  <li key={file} className="truncate">
                    {file}
                  </li>
                ))}
              </ul>
            )}
          </dd>
        </dl>
      )}
    </li>
  );
}

/** One field: its value, or why it's unavailable. What the agent says is
 * marked as its own words. */
function Field({ field, said = false }: { field: CardField; said?: boolean }): JSX.Element {
  if (field.value === null) return <span className="text-faint">{field.unavailable ?? "Unavailable"}</span>;
  return (
    <span className="text-ink">
      {said ? `“${field.value}”` : field.value}
      {said && <span className="ml-1.5 text-[11px] text-faint">said by the agent</span>}
    </span>
  );
}
