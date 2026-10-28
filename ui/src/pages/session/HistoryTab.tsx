/**
 * wf09, AJ-E1533, AJ-E2108 Req.4 — a session's History tab: "When ·
 * Event", oldest first, a page at a time (its status changes, its
 * lifecycle steps and the switches it took part in); then View Switch
 * Graph (its project's, when it switched) and Replay (read-only, once it
 * has ended).
 */
import { Suspense, useCallback, useState, type JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { PagedList } from "../../components/PagedList";
import { Button } from "../../components/primitives";
import { IconHistory } from "../../components/icons-places";
import { IconReplay } from "../../components/icons-actions";
import { isActiveStatus } from "../../components/indicator";
import { usePagedList } from "../../lib/paged";
import { sessionsHistory, type SessionEventInfo } from "../../lib/ipc/sessions";
import { historyWhen, historyWords } from "../../lib/session-history-words";
import { rememberTab } from "../registry";
import { lazyNamed } from "../../lib/lazy";
import type { InstanceInfo } from "../../lib/bridge";

// The replay loads when asked for (lazy-load rule).
const ReplayDialog = lazyNamed(() => import("../../components/ReplayDialog"), "ReplayDialog");

/** Scrolls the project's "How this work travelled" card into view once
 * the project page has drawn it (a few frames; gives up after 2 s). */
function showGraphWhenDrawn(started = Date.now()): void {
  const card = document.querySelector<HTMLElement>("[data-testid='project-switch-graph-card']");
  if (card) card.scrollIntoView({ block: "start" });
  else if (Date.now() - started < 2000) requestAnimationFrame(() => showGraphWhenDrawn(started));
}

export function SessionHistoryTab({ session, onOpenProject }: { session: InstanceInfo; onOpenProject: (id: string) => void }): JSX.Element {
  const fetchPage = useCallback((request: { cursor: string | null; limit: number | null }) => sessionsHistory(session.id, request), [session.id]);
  const list = usePagedList<SessionEventInfo>(`session-history:${session.id}`, fetchPage);
  const [replay, setReplay] = useState(false);
  const origin = { provider: session.provider, account: session.account_label };
  const switched = list.items.some((event) => event.kind === "switch");
  const ended = !isActiveStatus(session.status);
  const projectId = session.project_id;
  return (
    <Section title="History and lineage" icon={<IconHistory width={15} height={15} />} sub="what happened to this session, oldest first" testId="session-history">
      <PagedList
        list={list}
        keyOf={(event) => `${event.kind}:${event.at_ms}:${event.what}:${event.switch?.other_account ?? ""}`}
        label="This session's history"
        empty="Nothing recorded for this session yet."
        rowHeight={36}
        renderRow={(event, index) => (
          <div className="grid grid-cols-[8.5rem_minmax(0,1fr)] items-center gap-3 border-b border-hairline/50 py-2 text-body" data-testid="session-history-row" data-kind={event.kind}>
            <span className="tabular-nums text-muted">{historyWhen(event.at_ms)}</span>
            <span className="min-w-0 truncate text-ink">{historyWords(event, index === 0, origin)}</span>
          </div>
        )}
      />
      <div className="flex flex-wrap items-center gap-2">
        {switched && projectId && (
          <Button
            size="sm"
            variant="secondary"
            data-testid="session-history-graph"
            onClick={() => {
              // Its project's Overview holds the Switch Graph: open it there,
              // and bring the graph's card into view once it's drawn.
              rememberTab("project", "overview");
              onOpenProject(projectId);
              showGraphWhenDrawn();
            }}
          >
            View Switch Graph
          </Button>
        )}
        {ended ? (
          <Button size="sm" variant="secondary" data-testid="session-history-replay" onClick={() => setReplay(true)}>
            <IconReplay width={13} height={13} aria-hidden="true" />
            Replay (read-only)
          </Button>
        ) : (
          <span className="text-label text-muted" data-testid="session-history-replay-later">
            Replay opens once it ends; its terminal shows it now.
          </span>
        )}
      </div>
      {replay && (
        <Suspense fallback={null}>
          <ReplayDialog sessionId={session.id} onClose={() => setReplay(false)} />
        </Suspense>
      )}
    </Section>
  );
}
