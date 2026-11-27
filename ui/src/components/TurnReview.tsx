/**
 * AJ-E3820 — a turn's review, where the turn's end is shown (a finished
 * turn's feed post, the session page under its last turn), behind a
 * folded row like a turn's work (`TurnWork`): "Turn review · 2 files ·
 * Medium risk". Calm: depth on demand, never a ping, folded by default.
 * As the approved mock draws it (`AIJuggler-design-refs/turn-review`).
 *
 * What it shows is the code-only review (the backend's facts: the turn's
 * own edit calls and the agent's own sentences, never git). With the
 * review's Beta on and its model ready, opening it asks for the model's
 * lines after the facts are on screen (never blocking them): a short
 * description per changed file, each tagged AI, with whose lines they
 * are, the source one tap away (on the feed) and "This is wrong". While
 * they are written (a draft takes Apple's model about 2 s, the router's
 * budget at most 6 s, 0.5.2 G28) a quiet line says so.
 *
 * Nothing shows when there is no review (Last messages off, an App whose
 * turn ends aren't seen, no transcript here) or nothing in it (D-127).
 */
import { useCallback, useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { useAiJob } from "../lib/ai-availability";
import { reviewTurn, reviewTurnAi, reviewWrong, type ReviewAsk, type TurnReviewInfo } from "../lib/ipc/turn-review";
import { aiLines, foldWords, hasFindings, REVIEW, riskKey, whyWords } from "../lib/review-words";
import { FoldRow } from "../pages/session/TurnWork";

/** Each risk's words and mark (PAT-COLOR: red failed, amber needs a look,
 * green fine; never colour alone: the word goes with it). */
const RISK_LOOK = {
  high: { words: "text-danger", mark: "bg-danger-strong" },
  medium: { words: "text-warning", mark: "bg-warning-strong" },
  low: { words: "text-success", mark: "bg-success-strong" },
} as const;

const TESTS_LOOK: Record<string, keyof typeof RISK_LOOK> = { failed: "high", not_run: "medium", passed: "low" };

/** Asks for the review once the row is near the view (a page of posts
 * never reads every turn at once). */
export function useNear(): [(node: HTMLElement | null) => void, boolean] {
  const [near, setNear] = useState(false);
  const observer = useRef<IntersectionObserver | null>(null);
  const ref = useCallback((node: HTMLElement | null) => {
    observer.current?.disconnect();
    if (!node) return;
    if (typeof IntersectionObserver === "undefined") {
      setNear(true);
      return;
    }
    observer.current = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        setNear(true);
        observer.current?.disconnect();
      }
    }, { rootMargin: "300px" });
    observer.current.observe(node);
  }, []);
  return [ref, near];
}

function Mark({ risk, children, testId }: { risk: keyof typeof RISK_LOOK; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <span className={"inline-flex items-center gap-1.5 font-semibold " + RISK_LOOK[risk].words} data-testid={testId} data-risk={risk}>
      <span aria-hidden="true" className={"size-2 shrink-0 rounded-full " + RISK_LOOK[risk].mark} />
      {children}
    </span>
  );
}

function Quote({ children }: { children: ReactNode }): JSX.Element {
  return <blockquote className="mt-1 border-l-[3px] border-hairline pl-2.5 text-ink-2">“{children}”</blockquote>;
}

function Row({ label, children, testId }: { label: string; children: ReactNode; testId: string }): JSX.Element {
  return (
    <div className="grid grid-cols-[88px_minmax(0,1fr)] items-baseline gap-x-3" data-testid={testId}>
      <dt className="text-label text-faint">{label}</dt>
      <dd className="min-w-0 text-meta text-ink-2">{children}</dd>
    </div>
  );
}

export function AiTag(): JSX.Element {
  return <span className="shrink-0 rounded-[5px] bg-accent-soft px-1.5 text-label font-semibold text-accent-hi">{REVIEW.aiTag}</span>;
}

function Files({ review }: { review: TurnReviewInfo }): JSX.Element {
  return (
    <ul className="flex flex-col gap-1">
      {review.changed.map((file) => (
        <li key={file.name} className="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-2.5" data-testid="turn-review-file">
          <code className="truncate font-mono text-meta text-ink" title={file.name}>{file.name}</code>
          {file.added !== null && file.removed !== null ? (
            <span className="whitespace-nowrap text-label tabular-nums">
              <span className="font-semibold text-success">+{file.added}<span className="sr-only"> {file.added === 1 ? "line" : "lines"} added</span></span>{" "}
              <span className="font-semibold text-danger">−{file.removed}<span className="sr-only"> {file.removed === 1 ? "line" : "lines"} removed</span></span>
            </span>
          ) : (
            <span />
          )}
          {file.about && (
            <span className="col-span-2 mb-0.5 flex items-baseline gap-1.5 text-meta text-muted" data-testid="turn-review-about">
              <AiTag />
              {file.about}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

/** Whose the model's lines are, the source, and "This is wrong". */
function AiFoot({ review, quote }: { review: TurnReviewInfo; quote: boolean }): JSX.Element | null {
  const [source, setSource] = useState(false);
  const [wrong, setWrong] = useState(false);
  const ai = review.ai;
  if (!ai) return null;
  const mark = (on: boolean) => {
    setWrong(on);
    void reviewWrong(review.turn, aiLines(review), on);
  };
  const link = "rounded text-label text-muted outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";
  return (
    <div className="flex flex-col gap-2 border-t border-divider pt-2" data-testid="turn-review-ai">
      <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1 text-label text-faint">
        <AiTag />
        <span>{REVIEW.aiBy(ai.model)}</span>
        <span className="rounded-full bg-accent-soft px-1.5 font-medium text-accent-hi">{REVIEW.beta}</span>
        <span className="flex-1" />
        {quote && ai.source && (
          <button type="button" aria-expanded={source} onClick={() => setSource((was) => !was)} className={link} data-testid="turn-review-source-toggle">
            {source ? REVIEW.hideWords : REVIEW.showWords}
          </button>
        )}
        {wrong ? (
          <output data-testid="turn-review-wrong-kept">
            {REVIEW.wrongKept}{" "}
            <button type="button" onClick={() => mark(false)} className={link}>{REVIEW.undo}</button>
          </output>
        ) : (
          <button type="button" onClick={() => mark(true)} className={link} data-testid="turn-review-wrong">
            {REVIEW.wrong}
          </button>
        )}
      </div>
      {source && (
        <figure className="m-0" data-testid="turn-review-source">
          <figcaption className="text-label text-faint">{REVIEW.wordsLabel}</figcaption>
          <p className="mt-1 whitespace-pre-wrap text-meta text-ink-2">{ai.source}</p>
        </figure>
      )}
    </div>
  );
}

/** 0.5.2 G28: the model's lines are on their way (they come after the
 * code-only review, never in its place; a refused draft never shows, so
 * this goes without a word when none passes). */
function Writing({ model }: { model: string | null }): JSX.Element {
  return (
    <output className="flex items-center gap-2 border-t border-divider pt-2 text-label text-faint" data-testid="turn-review-writing">
      <AiTag />
      <span aria-hidden="true" className="size-1.5 shrink-0 rounded-full bg-accent motion-safe:animate-pulse" />
      <span>{REVIEW.writing(model)}</span>
    </output>
  );
}

/** The opened review. `quote`: the agent's sentences are quoted (the
 * feed); on the session page the reply is right above, so they're named. */
function Panel({ review, quote, writing }: { review: TurnReviewInfo; quote: boolean; /** The model's lines are on their way, by this model. */ writing: string | false | null }): JSX.Element {
  const risk = riskKey(review);
  const tests = review.tests ? TESTS_LOOK[review.tests] : undefined;
  const why = whyWords(review);
  return (
    <div className="mb-2 ml-1.5 mt-1 flex flex-col gap-2.5 border-l border-hairline py-0.5 pl-3.5" data-testid="turn-review-panel">
      <dl className="flex flex-col gap-2.5">
      <Row label={REVIEW.label.risk} testId="turn-review-risk">
        <Mark risk={risk}>{REVIEW.riskWord[risk]}</Mark>
        {why && <span className="text-muted"> · {why}</span>}
        {review.ai?.risk && <span className="text-muted"> · {REVIEW.aiRisk(review.ai.risk)}</span>}
        {review.ai?.risk_reason && (
          <span className="mt-0.5 flex items-baseline gap-1.5 text-muted" data-testid="turn-review-risk-reason">
            <AiTag />
            {review.ai.risk_reason}
          </span>
        )}
      </Row>
      {review.changed.length > 0 && (
        <Row label={review.undone_unclear ? REVIEW.label.edited : REVIEW.label.changed} testId="turn-review-changed">
          <Files review={review} />
          {review.undone_unclear && <p className="mt-1 text-muted">{REVIEW.unclear}</p>}
        </Row>
      )}
      {(review.reverted.length > 0 || (review.reverted_words && !review.undone_unclear)) && (
        <Row label={REVIEW.label.reverted} testId="turn-review-reverted">
          {review.reverted.length > 0 && <code className="block truncate font-mono text-meta text-ink">{review.reverted.join(", ")}</code>}
          {quote && review.reverted_words && <Quote>{review.reverted_words}</Quote>}
        </Row>
      )}
      {tests && review.tests && (
        <Row label={REVIEW.label.tests} testId="turn-review-tests">
          <Mark risk={tests}>{REVIEW.tests[review.tests]}</Mark>
          {quote && review.tests_words ? <Quote>{review.tests_words}</Quote> : <span className="text-muted">, {REVIEW.asItSays}</span>}
        </Row>
      )}
      {review.failures.length > 0 && (
        <Row label={REVIEW.label.failed} testId="turn-review-failures">
          {review.failures.map((failure) => (
            <Quote key={failure}>{failure}</Quote>
          ))}
        </Row>
      )}
      {review.question && (
        <Row label={REVIEW.label.asks} testId="turn-review-question">
          {quote ? (
            <Quote>{review.question}</Quote>
          ) : (
            <span>
              <span className="font-semibold text-warning">{REVIEW.yes}</span>
              <span className="text-muted"> · {REVIEW.askedAbove}</span>
            </span>
          )}
        </Row>
      )}
      </dl>
      {writing !== false && !review.ai && <Writing model={writing} />}
      <AiFoot review={review} quote={quote} />
    </div>
  );
}

/** A stable key for an ask (its fields, never the object). */
function keyOf(ask: ReviewAsk): string {
  return [ask.session_id, ask.account_id, ask.conversation_id, ask.until_ms, ask.ended].join("\u0000");
}

export function TurnReview({ ask, quote, card = false }: { ask: ReviewAsk; quote: boolean; /** On a card of its own (under a replayed terminal, G28), only once there is a review. */ card?: boolean }): JSX.Element | null {
  const key = keyOf(ask);
  const [ref, near] = useNear();
  const [review, setReview] = useState<TurnReviewInfo | null>(null);
  // The reader's choice, folded by default.
  const [open, setOpen] = useState(false);
  const job = useAiJob("review");
  const aiReady = job?.state === "ready";
  const asked = useRef<string | null>(null);
  // G28: the turn whose lines are being written, while they are.
  const [writing, setWriting] = useState<string | null>(null);
  useEffect(() => {
    let live = near;
    if (live) {
      setReview(null);
      reviewTurn(ask).then(
        (got) => live && setReview(got),
        () => live && setReview(null),
      );
    }
    return () => {
      live = false;
    };
    // `key` stands for `ask`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [near, key]);
  useEffect(() => {
    let live = open && aiReady && review?.state === "shown" && !review.ai && asked.current !== review.turn;
    if (live && review) {
      const turn = review.turn;
      asked.current = turn;
      setWriting(turn);
      const done = () => setWriting((was) => (was === turn ? null : was));
      reviewTurnAi(ask).then(
        (got) => {
          if (live && got.ai && got.turn === turn) setReview(got);
          done();
        },
        done,
      );
    }
    return () => {
      live = false;
    };
    // `key` stands for `ask`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, aiReady, review, key]);
  const shown = review !== null && review.state === "shown" && hasFindings(review);
  if (!shown) return <span ref={ref} aria-hidden="true" className="block h-0" data-testid="turn-review-probe" />;
  const risk = riskKey(review);
  return (
    <div className={"flex flex-col" + (card ? " rounded-card border border-frame bg-surface px-4 py-2" : "")} data-testid="turn-review" data-risk={risk} data-ai={review.ai ? "yes" : undefined}>
      <FoldRow open={open} onToggle={() => setOpen((was) => !was)} testId="turn-review-toggle">
        {foldWords(review)} · <span className={"font-semibold " + RISK_LOOK[risk].words}>{REVIEW.risk[risk]}</span>
      </FoldRow>
      {open && <Panel review={review} quote={quote} writing={writing === review.turn ? job?.model_name ?? null : false} />}
    </div>
  );
}
