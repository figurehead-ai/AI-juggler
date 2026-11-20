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
 * SK-5: one that ended in an error says why, where its ending knows, and
 * offers to start it again in its folder.
 * The terminal stays mounted while it ends and while that's asked (one
 * that ends as you watch is never rebuilt). One opened ended shows a
 * skeleton while that's asked, never a blank terminal (the 0.4.6 RC walk
 * K2, F-K2-6: black for about a second under load), and the answer is
 * kept, so opening it again shows what it kept at once.
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";
import { Button, Card } from "../../components/primitives";
import { terminalKept } from "../../lib/ipc/terminal-io";
import { providerDisplayName } from "../../lib/providers";
import { pastWhen, since } from "../../lib/time-words";
import { endedCause } from "../../lib/ended-how";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { LastMessages } from "./LastMessages";
import { fromInstance } from "./model";

/** What each ended session's terminal kept, by `id@last activity`: an
 * ended one's kept output only changes when it runs again (its last
 * activity moves), and the window's memory goes with the app's. */
const answered = new Map<string, boolean>();

/** Forgets the answers (tests). */
export function forgetKept(): void {
  answered.clear();
}

/** Whether the terminal `key` names kept output to replay: the kept
 * answer at once, else `null` while asked (and for no `key`: one still
 * running). */
function useKept(id: string | null, key: string | null): boolean | null {
  const [answer, setAnswer] = useState<{ key: string; kept: boolean } | null>(null);
  useEffect(() => {
    if (id === null || key === null || answered.has(key)) return undefined;
    let alive = true;
    void terminalKept(id).then((kept) => {
      answered.set(key, kept);
      if (alive) setAnswer({ key, kept });
    });
    return () => {
      alive = false;
    };
  }, [id, key]);
  if (key === null) return null;
  return answered.get(key) ?? (answer?.key === key ? answer.kept : null);
}

/** The terminal's shape while what it kept is asked: never a blank pane. */
function Asking(): JSX.Element {
  return (
    <Card className="flex min-h-[280px] flex-1 flex-col p-5" data-testid="ended-terminal-asking">
      <output aria-label="Reading what its terminal kept" className="flex flex-col gap-3">
        <span className="h-3 w-1/3 rounded bg-skeleton motion-safe:animate-pulse" />
        <span className="h-3 w-2/3 rounded bg-skeleton motion-safe:animate-pulse" />
        <span className="h-3 w-1/2 rounded bg-skeleton motion-safe:animate-pulse" />
      </output>
    </Card>
  );
}

/** History, searching for `words` (the rail's "Search all history"). */
function searchHistory(words: string): void {
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history", query: words } }));
}

/** SK-5 (the 0.4.6 soak, K3: "Error 8:35 AM" and "No output was kept", no
 * why): one that ended in an error or a crash says why where its ending
 * knows ("Claude Code exited with code 1, at 8:35 AM."), else when, and
 * that its tool didn't say; `null` for any other ending. */
function errorWords(session: InstanceInfo, now: number): { title: string; why: string } | null {
  const how = session.ending?.how;
  if (how !== "error" && how !== "crashed") return null;
  const tool = providerDisplayName(session.provider);
  const at = pastWhen(session.last_activity_ms ?? session.started_at_ms, now);
  const cause = endedCause(session.ending?.detail, tool);
  return { title: how === "crashed" ? "It crashed" : "It stopped with an error", why: cause ? `${cause}, at ${at}.` : `It stopped at ${at}; ${tool} didn't say why.` };
}

/** New session in its folder on its App: run again, the error shows as it happens. */
function startAgain(session: InstanceInfo): void {
  window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: session.account_id, dir: session.cwd } }));
}

export function SessionTerminal({ session, live, terminal, now }: { session: InstanceInfo; live: boolean; terminal: ReactNode; now: number }): JSX.Element {
  const kept = useKept(live ? null : session.id, live ? null : `${session.id}@${session.last_activity_ms ?? session.started_at_ms}`);
  // Seen running here: its terminal stays while what it kept is asked.
  const [seenLive, setSeenLive] = useState(live);
  if (live && !seenLive) setSeenLive(true);
  if (!live && kept === null && !seenLive) return <Asking />;
  if (live || kept !== false) {
    return (
      <div className="flex min-h-0 flex-1 flex-col gap-2" data-testid={live ? undefined : kept ? "ended-terminal-kept" : "ended-terminal-asked-live"}>
        {terminal}
        {!live && kept && <p className="text-meta text-ink-2">{`ran ${since(session.started_at_ms, now)} · scrollback replayed`}</p>}
      </div>
    );
  }
  const failed = errorWords(session, now);
  if (session.conversation_id) {
    return (
      <div className="flex min-h-0 flex-1 flex-col gap-2" data-testid="ended-terminal-conversation">
        {failed && (
          <p className="text-meta text-ink" data-testid="ended-why">
            {failed.why}
          </p>
        )}
        <p className="text-meta text-ink-2">Its terminal output wasn't kept, so this is its conversation.</p>
        <LastMessages model={fromInstance(session)} visible />
      </div>
    );
  }
  const tool = providerDisplayName(session.provider);
  return (
    <Card className="flex min-h-[280px] flex-1 flex-col items-center justify-center gap-3 p-6 text-center" data-testid="ended-terminal-none">
      <p className="text-section font-semibold text-ink">{failed?.title ?? "No output was kept"}</p>
      {failed && (
        <p className="max-w-md text-body text-ink" data-testid="ended-why">
          {failed.why}
        </p>
      )}
      <p className="max-w-md text-body text-ink-2">
        {failed
          ? `No output was kept: AI Juggler keeps a terminal's output only while it runs, and ${tool} recorded no conversation. Start it again in the same folder to see what it says as it happens.`
          : `AI Juggler keeps a terminal's output only while it runs, and ${tool} recorded no conversation for this session.`}
      </p>
      <div className="flex flex-wrap justify-center gap-2">
        {failed && (
          <Button size="sm" variant="primary" onClick={() => startAgain(session)} data-testid="ended-start-again">
            Start again here
          </Button>
        )}
        <Button size="sm" variant="secondary" onClick={() => searchHistory(session.label ?? "")} data-testid="ended-terminal-history">
          Search History
        </Button>
      </div>
    </Card>
  );
}
