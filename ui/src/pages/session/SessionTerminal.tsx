/**
 * P2 (the D3 persona walk) — the main pane of a session AI Juggler runs in
 * its terminal. A terminal's output is kept in memory only while AI
 * Juggler runs, so one reopened days later, or after a restart, showed an
 * empty terminal under "scrollback replayed".
 *
 * Running, its terminal. Ended, what there is: the terminal, replayed,
 * and that line only when output was kept; else its conversation, read
 * from the tool's own transcript as every session's page shows it; else
 * (no conversation either) that nothing was kept, and History's search.
 * The terminal stays mounted while it ends and while that's asked (one
 * that ends as you watch is never rebuilt).
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";
import { Button, Card } from "../../components/primitives";
import { terminalKept } from "../../lib/ipc/terminal-io";
import { providerDisplayName } from "../../lib/providers";
import { since } from "../../lib/time-words";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { LastMessages } from "./LastMessages";
import { fromInstance } from "./model";

/** Whether `id`'s terminal kept output to replay; `null` while asked
 * (and for no `id`: one still running). */
function useKept(id: string | null): boolean | null {
  const [answer, setAnswer] = useState<{ id: string; kept: boolean } | null>(null);
  useEffect(() => {
    if (id === null) return undefined;
    let alive = true;
    void terminalKept(id).then((kept) => {
      if (alive) setAnswer({ id, kept });
    });
    return () => {
      alive = false;
    };
  }, [id]);
  return answer?.id === id ? answer.kept : null;
}

/** History, searching for `words` (the rail's "Search all history"). */
function searchHistory(words: string): void {
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history", query: words } }));
}

export function SessionTerminal({ session, live, terminal, now }: { session: InstanceInfo; live: boolean; terminal: ReactNode; now: number }): JSX.Element {
  const kept = useKept(live ? null : session.id);
  if (live || kept !== false) {
    return (
      <div className="flex min-h-0 flex-1 flex-col gap-2" data-testid={live ? undefined : kept ? "ended-terminal-kept" : "ended-terminal-asking"}>
        {terminal}
        {!live && kept && <p className="text-meta text-ink-2">{`ran ${since(session.started_at_ms, now)} · scrollback replayed`}</p>}
      </div>
    );
  }
  if (session.conversation_id) {
    return (
      <div className="flex min-h-0 flex-1 flex-col gap-2" data-testid="ended-terminal-conversation">
        <p className="text-meta text-ink-2">Its terminal output wasn't kept, so this is its conversation.</p>
        <LastMessages model={fromInstance(session)} visible />
      </div>
    );
  }
  const tool = providerDisplayName(session.provider);
  return (
    <Card className="flex min-h-[280px] flex-1 flex-col items-center justify-center gap-3 p-6 text-center" data-testid="ended-terminal-none">
      <p className="text-section font-semibold text-ink">No output was kept</p>
      <p className="max-w-md text-body text-ink-2">{`AI Juggler keeps a terminal's output only while it runs, and ${tool} recorded no conversation for this session.`}</p>
      <Button size="sm" variant="secondary" onClick={() => searchHistory(session.label ?? "")} data-testid="ended-terminal-history">
        Search History
      </Button>
    </Card>
  );
}
