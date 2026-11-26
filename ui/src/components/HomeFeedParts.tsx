/**
 * AJ-E3713 R2 — a post's parts, as the approved mock draws them
 * (`AIJuggler-design-refs/story-feed`): the author's picture (the App's
 * own icon, read at run time as the rail's and Your apps' are, or AI
 * Juggler's mark on its own tile), the story with its things as chips,
 * what it shows beside the words, and its thread.
 *
 * The chip is one unbroken, left-aligned box (0.5.1 Nightly: the ChatGPT ·
 * Codex row's bold title, a button, wrapped inside itself and centred
 * mid-row); past its width it ends in "…", its whole name in its tooltip.
 */
import type { JSX } from "react";
import { AppIconTile } from "./AppIcon";
import { IconJuggler } from "./icons";
import { appOfTool } from "../lib/app-of";
import { clock } from "../lib/time-words";
import { followUp, laterWords, type Attachment, type PostAuthor, type StoryPart } from "../lib/feed-story";
import type { FeedItemInfo } from "../lib/ipc/feed";

/** The post's look: a surface on the canvas (no border, no shadow,
 * PAT-SURF-1/-2); a keyboard ring when its row (Open) has focus. */
export const POST =
  "group/card tone-host relative grid min-w-0 grid-cols-[40px_minmax(0,1fr)] gap-x-3 rounded-[14px] bg-surface px-4 pb-2 pt-3.5 outline-none has-[[data-home-row]:focus-visible]:ring-1 has-[[data-home-row]:focus-visible]:ring-accent/60";

/** The author's picture, 40 px (the mock's). */
export function PostAvatar({ item, author }: { item: FeedItemInfo; author: PostAuthor }): JSX.Element {
  if (author.kind === "app" && item.author.provider) {
    const app = appOfTool(item.author.provider);
    return <AppIconTile appId={app.appId} appPid={null} name={author.name} size={40} />;
  }
  return (
    <span aria-hidden="true" className="flex size-10 shrink-0 items-center justify-center rounded-[11px] bg-surface-3" data-testid="post-mark">
      <IconJuggler width={26} height={26} />
    </span>
  );
}

/** The chip's look: the accent's soft step, its words never wrapping. */
const CHIP =
  "inline-block max-w-[min(100%,22rem)] truncate whitespace-nowrap rounded-md bg-accent-soft px-1.5 text-left align-bottom font-semibold text-accent-hi outline-none hover:bg-accent/20 focus-visible:ring-1 focus-visible:ring-accent/60";

/** A story's parts: a chip opens its thing. */
export function Story({ parts, onOpen, onPool }: { parts: readonly StoryPart[]; onOpen: () => void; onPool: () => void }): JSX.Element {
  return (
    <>
      {parts.map((part, at) => {
        if (part.chip) {
          return (
            <button key={at} type="button" title={part.text} onClick={part.chip === "pool" ? onPool : onOpen} className={CHIP} data-testid={part.chip === "pool" ? "feed-pool" : "feed-subject"}>
              {part.text}
            </button>
          );
        }
        return part.strong ? <strong key={at} className="font-semibold text-ink">{part.text}</strong> : <span key={at}>{part.text}</span>;
      })}
    </>
  );
}

/** An App's small mark in a switch's before → after. */
function AppSide({ label, provider }: { label: string; provider: string | null }): JSX.Element {
  const app = provider ? appOfTool(provider) : null;
  return (
    <span className="inline-flex min-w-0 items-center gap-2 rounded-lg bg-surface px-2.5 py-1.5 text-ink">
      {app ? <AppIconTile appId={app.appId} appPid={null} name={label} size={16} /> : null}
      <span className="truncate">{label}</span>
    </span>
  );
}

/** Room used, as a small meter: the strong amber step under a fifth. */
function UsageMeter({ left, window, resetsAt }: { left: number; window: string | null; resetsAt: number | null }): JSX.Element {
  const used = Math.max(0, Math.min(100, 100 - left));
  const low = left < 20;
  return (
    <div className="flex items-center gap-4" data-testid="feed-usage">
      <span aria-hidden="true" className="flex h-[34px] w-24 items-end overflow-hidden rounded-sm bg-surface-3">
        <span className={"block w-full rounded-t-sm " + (low ? "bg-warning" : "bg-accent/55")} style={{ height: `${Math.max(4, used)}%` }} />
      </span>
      <span className="flex flex-col">
        <b className={low ? "font-semibold text-warning" : "font-semibold text-ink"}>{used}% used</b>
        <span className="text-faint">{[window ? `${window} window` : null, resetsAt ? `resets ${clock(resetsAt)}` : null].filter(Boolean).join(" · ")}</span>
      </span>
    </div>
  );
}

/** What a post shows beside its words. */
export function PostAttachment({ attachment }: { attachment: Attachment }): JSX.Element {
  const look = "mb-2 mt-0.5 rounded-[10px] bg-surface-2 px-3 py-2.5 text-ink-2";
  if (attachment.kind === "switch") {
    return (
      <div className={look + " flex flex-wrap items-center gap-2.5"} data-testid="feed-switch">
        <AppSide {...attachment.from} />
        <span aria-label="to">→</span>
        <AppSide {...attachment.to} />
      </div>
    );
  }
  if (attachment.kind === "quote") {
    return (
      <div className={look} data-testid="feed-quote">
        <blockquote className="line-clamp-4 border-l-[3px] border-hairline pl-2.5 text-ink-2">“{attachment.text}”</blockquote>
        <p className="mt-1 pl-[13px] text-label text-faint">{attachment.who}</p>
      </div>
    );
  }
  return (
    <div className={look}>
      <UsageMeter left={attachment.left} window={attachment.window} resetsAt={attachment.resetsAt} />
    </div>
  );
}

/** A post's thread: what came next for the same thing, oldest first. A
 * follow-up that broke or needs the reader always shows; the rest past
 * the newest three fold into one line. */
export function PostThread({ from, follow, shownAll, onAll, onOpen, onPool }: {
  from: number;
  follow: readonly FeedItemInfo[];
  shownAll: boolean;
  onAll: () => void;
  /** Opens a follow-up's thing, or its pool. */
  onOpen: (item: FeedItemInfo) => void;
  onPool: (item: FeedItemInfo) => void;
}): JSX.Element {
  const newest = follow.slice(-3);
  const shown = shownAll ? follow : follow.filter((item) => newest.includes(item) || item.level >= 2);
  const hidden = follow.length - shown.length;
  return (
    <div className="mb-2 ml-0.5 flex flex-col gap-0.5 border-l-2 border-divider py-0.5 pl-3 text-ink-2" data-testid="feed-thread">
      {hidden > 0 && (
        <button type="button" onClick={onAll} className="self-start rounded text-label text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-thread-all">
          {hidden} earlier {hidden === 1 ? "update" : "updates"}
        </button>
      )}
      {shown.map((item) => (
        <p key={item.id} className="min-w-0" data-testid="feed-followup" data-level={item.level}>
          <span className="text-faint">{laterWords(item.at_ms, from)}</span> · <Story parts={followUp(item)} onOpen={() => onOpen(item)} onPool={() => onPool(item)} />
        </p>
      ))}
    </div>
  );
}
