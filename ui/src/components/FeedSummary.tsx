/**
 * AJ-E3831 (D-154) — a finished turn's AI summary on its feed post: what
 * the turn came to against what was asked, in one line, like someone
 * watching the agent and telling you. An opt-in Beta ("AI summaries in
 * the feed" in Settings › AI › Models); off, nothing here shows or runs.
 *
 *   [AI] Partly done: added the priority option · open: calendar sync
 *   Summary by Apple's on-device model · Beta   Show its words · This is wrong
 *
 * The agent's own words are one tap away ("Show its words"), and stand in
 * the line's place when every draft was refused. Asked once the post is
 * near the view (a page of posts never runs the model for every turn at
 * once); the post never waits on it.
 */
import { useEffect, useState, type JSX } from "react";
import { isShown, useAiJob } from "../lib/ai-availability";
import { feedLine, feedLineWrong, type FeedLineInfo } from "../lib/ipc/feed-line";
import type { ReviewAsk } from "../lib/ipc/turn-review";
import { REVIEW } from "../lib/review-words";
import { AiTag, useNear } from "./TurnReview";

const LINK = "rounded text-label text-muted outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";

function Words({ text, label }: { text: string; label: string }): JSX.Element {
  return (
    <figure className="m-0" data-testid="feed-summary-words">
      <figcaption className="text-label text-faint">{label}</figcaption>
      <blockquote className="mt-1 border-l-[3px] border-hairline pl-2.5 text-meta text-ink-2">“{text}”</blockquote>
    </figure>
  );
}

function Line({ info }: { info: FeedLineInfo }): JSX.Element {
  const [words, setWords] = useState(false);
  const [wrong, setWrong] = useState(false);
  const mark = (on: boolean) => {
    setWrong(on);
    void feedLineWrong(info.turn, info.line ?? "", on);
  };
  return (
    <div className="mb-2 flex flex-col gap-1.5" data-testid="feed-summary">
      <p className="flex items-baseline gap-2 text-body text-ink">
        <AiTag />
        <span data-testid="feed-summary-line">{info.line}</span>
      </p>
      <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1 text-label text-faint">
        {info.model && <span>{REVIEW.summaryBy(info.model)}</span>}
        <span className="rounded-full bg-accent-soft px-1.5 font-medium text-accent-hi">{REVIEW.beta}</span>
        <span className="flex-1" />
        {info.excerpt && (
          <button type="button" aria-expanded={words} onClick={() => setWords((was) => !was)} className={LINK} data-testid="feed-summary-words-toggle">
            {words ? REVIEW.hideWords : REVIEW.showWords}
          </button>
        )}
        {wrong ? (
          <output data-testid="feed-summary-wrong-kept">
            {REVIEW.wrongKept}{" "}
            <button type="button" onClick={() => mark(false)} className={LINK}>{REVIEW.undo}</button>
          </output>
        ) : (
          <button type="button" onClick={() => mark(true)} className={LINK} data-testid="feed-summary-wrong">
            {REVIEW.wrong}
          </button>
        )}
      </div>
      {words && info.excerpt && <Words text={info.excerpt} label={REVIEW.summaryWords} />}
    </div>
  );
}

/** The summary of the post's turn, while its Beta runs. */
export function FeedSummary({ ask }: { ask: ReviewAsk }): JSX.Element | null {
  const ready = isShown(useAiJob("feed"));
  const [ref, near] = useNear();
  const [info, setInfo] = useState<FeedLineInfo | null>(null);
  const key = `${ask.session_id ?? ""}|${ask.conversation_id ?? ""}|${ask.until_ms ?? ""}`;
  useEffect(() => {
    let live = ready && near;
    if (live) {
      setInfo(null);
      feedLine(ask).then(
        (got) => live && setInfo(got),
        () => live && setInfo(null),
      );
    }
    return () => {
      live = false;
    };
    // `key` stands for `ask`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, near, key]);
  if (!ready) return null;
  if (info?.state === "shown" && info.line) return <Line info={info} />;
  // A refused draft: the agent's own words in the line's place.
  if (info?.state === "plain" && info.excerpt) {
    return (
      <div className="mb-2" data-testid="feed-summary-plain">
        <Words text={info.excerpt} label={REVIEW.lastReply} />
      </div>
    );
  }
  return <span ref={ref} aria-hidden="true" className="block h-0" data-testid="feed-summary-probe" />;
}
