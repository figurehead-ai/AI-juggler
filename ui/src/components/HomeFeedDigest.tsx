/**
 * AJ-E3712 (D-132, ADR-0036 §1.3–§1.4) — the agent digest in Home's feed:
 * a few short lines saying what each agent is on and what is notable,
 * needs and failures first, every changed agent accounted for.
 *
 *   [◎]  AI Juggler · Digest                                 10:45 AM
 *        5 agents
 *        [▣] web · failed: “The build failed: missing module auth.”
 *        [▣] api · asks you: “Should I bump the major version?”
 *        [▣] docs · finished · tests passed (52)
 *        and 2 more agents · 3 quiet
 *
 * Calm (D-126): normal ink, no amber or red, even for a need (its loud
 * copy is the pinned zone's, D-117). Each name opens its session. The
 * words are `feed-words.ts`'s; the rollup's facts are the backend's.
 */
import { useState, type JSX } from "react";
import { CliAppMark } from "./PlaceMarks";
import { IconJuggler } from "./icons";
import { digestHeader, digestLineWords, digestMoreWords, exactWhen } from "../lib/feed-words";
import type { FeedItemInfo } from "../lib/ipc/feed";
import type { DigestInfo } from "../types/generated/DigestInfo";
import type { DigestLineInfo } from "../types/generated/DigestLineInfo";

function Mark({ line }: { line: DigestLineInfo }): JSX.Element {
  const { provider, folder, folder_label } = line.author;
  if (provider && folder) {
    return <span className="app-tone inline-flex shrink-0"><CliAppMark tool={provider} folderId={folder} folder={folder_label ?? ""} size={14} ring="var(--color-canvas)" /></span>;
  }
  return <span aria-hidden="true" className="size-3.5 shrink-0 rounded-full bg-surface-3" />;
}

function Line({ line, onOpen }: { line: DigestLineInfo; onOpen: (sessionId: string) => void }): JSX.Element {
  const words = digestLineWords(line);
  return (
    <li className="flex min-w-0 items-baseline gap-2 text-meta text-ink-2" data-testid="digest-line" data-level={line.level}>
      <Mark line={line} />
      <span className="min-w-0">
        <button type="button" onClick={() => onOpen(line.session_id)} className="rounded font-semibold text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">{line.name}</button>
        <span>{words.state}</span>
        {words.said && <span>{words.said}</span>}
        {words.facts && <span className="text-muted">{words.facts}</span>}
        {words.last && <span className="text-muted">{words.last}</span>}
      </span>
    </li>
  );
}

/** The digest's body: its header, its first lines, the rest folded. */
export function HomeFeedDigest({ item, digest, when, onOpen }: { item: FeedItemInfo; digest: DigestInfo; when: string; onOpen: (sessionId: string) => void }): JSX.Element {
  const [all, setAll] = useState(false);
  const shown = all ? digest.lines : digest.lines.slice(0, digest.shown);
  const more = digestMoreWords(all ? 0 : digest.lines.length - digest.shown, digest.quiet);
  const id = `feed-${item.id.replace(/[^a-zA-Z0-9_-]/g, "_")}`;
  return (
    <article
      aria-labelledby={`${id}-who ${id}-what`}
      tabIndex={-1}
      data-feed-item
      data-home-item
      data-testid="feed-digest"
      data-kind={item.kind}
      data-feed-ids={item.id}
      className="group/card tone-host relative flex min-w-0 gap-3 rounded-lg px-3 py-2.5 outline-none hover:bg-surface/70 focus-within:bg-surface/70 focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      <span aria-hidden="true" className="app-tone mt-0.5 flex size-5 shrink-0 items-center justify-center text-muted"><IconJuggler width={16} height={16} /></span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <p className="flex min-w-0 items-baseline gap-1.5 text-meta">
          <span id={`${id}-who`} className="truncate font-medium text-ink-2">AI Juggler · Digest</span>
          <time className="ml-auto shrink-0 pl-2 text-label text-muted" dateTime={new Date(item.at_ms).toISOString()} title={exactWhen(item.at_ms)} aria-label={exactWhen(item.at_ms)}>{when}</time>
        </p>
        <p id={`${id}-what`} className="text-[0.84375rem] leading-snug text-ink" data-testid="digest-header">{digestHeader(digest)}</p>
        <ul className="mt-1 flex flex-col gap-1" data-testid="digest-lines">
          {shown.map((line) => <Line key={line.session_id} line={line} onOpen={onOpen} />)}
        </ul>
        {more && (
          digest.lines.length > digest.shown && !all ? (
            <button type="button" aria-expanded={false} onClick={() => setAll(true)} className="self-start rounded text-label text-ink-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="digest-more">{more}</button>
          ) : (
            <p className="text-label text-muted" data-testid="digest-quiet">{more}</p>
          )
        )}
      </div>
    </article>
  );
}
