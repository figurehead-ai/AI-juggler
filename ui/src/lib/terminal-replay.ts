/**
 * SK-23 (the 0.4.7 persona runs): a terminal's view shows every byte its
 * session printed once, in order, however it opens. Two streams fill it:
 * the output AI Juggler keeps (asked for when the view opens) and the live
 * chunks (heard once its listener is in place). Each says where it ends in
 * the terminal's whole output, so they are joined by place: a chunk the
 * kept output already holds is dropped, one that leaves a gap after what's
 * shown asks for the kept bytes after it first. Asked in parallel, nothing
 * printed between the two is lost (a session that asks a question once and
 * waits showed a blank terminal), and nothing shows twice.
 */

/** Bytes of a terminal's output, and where they end in it. */
export type Span = { bytes: Uint8Array; end: number };

export interface Replay {
  /** The kept output (all of it, or what's kept after a place). */
  kept(span: Span): void;
  /** A live chunk. */
  heard(span: Span): void;
  /** The live stream is in place (its listener, and the backend's
   * forwarding): what was printed before it is kept output, so the view
   * asks for what it hasn't shown of that. */
  listening(): void;
  /** An ask for kept output failed: what's heard is shown from here on. */
  lost(): void;
}

/** A view's join: `write` shows bytes, `ask(from)` asks for the kept
 * output after place `from` (its answer comes back as `kept`). */
export function replay(write: (bytes: Uint8Array) => void, ask: (from: number) => void): Replay {
  /** Where what's shown ends; `null` until the kept output comes. */
  let shown: number | null = null;
  let asking = false;
  /** The live stream came while nothing was shown yet, or while an ask
   * made before it was out: ask once more after that answer. */
  let catchUp = false;
  const waiting: Span[] = [];
  const startOf = (span: Span) => span.end - span.bytes.length;
  const askAfter = (from: number) => {
    if (asking) return;
    asking = true;
    ask(from);
  };
  /** Shows the part of `span` past what's shown; false when a gap is before it. */
  const put = (span: Span, at: number): boolean => {
    if (span.end <= at) return true;
    const start = startOf(span);
    if (start > at) return false;
    write(start === at ? span.bytes : span.bytes.subarray(at - start));
    shown = span.end;
    return true;
  };
  /** Shows what's waiting, in order, up to a gap (asked for). */
  const drain = (): void => {
    for (let next = waiting[0]; next; next = waiting[0]) {
      if (shown === null || asking) return;
      if (!put(next, shown)) return askAfter(shown);
      waiting.shift();
    }
  };
  return {
    kept(span) {
      asking = false;
      // The first, or bytes between were let go before they were asked
      // for (the kept tail is bounded): what's kept from here.
      if (shown === null || startOf(span) > shown) {
        if (span.bytes.length > 0) write(span.bytes);
        shown = span.end;
      } else put(span, shown);
      if (catchUp) {
        catchUp = false;
        askAfter(shown);
      }
      drain();
    },
    heard(span) {
      waiting.push(span);
      drain();
    },
    listening() {
      if (shown === null || asking) catchUp = true;
      else askAfter(shown);
    },
    lost() {
      asking = false;
      const next = waiting[0];
      if (next && shown !== null) shown = Math.max(shown, startOf(next));
      drain();
    },
  };
}
