/**
 * SK-23: a terminal's view joins the kept output and the live chunks by
 * place, so every byte shows once, in order, whichever comes first.
 */
import { describe, expect, it } from "vitest";
import { replay, type Span } from "./terminal-replay";

const enc = new TextEncoder();
const dec = new TextDecoder();
/** The output `text` holds from place `from` on. */
const span = (text: string, from: number): Span => ({ bytes: enc.encode(text), end: from + enc.encode(text).length });

/** A terminal whose whole output is `output`, and a view joining it. */
function view(output: string) {
  let shown = "";
  const asks: number[] = [];
  const join = replay(
    (bytes) => (shown += dec.decode(bytes)),
    (from) => asks.push(from),
  );
  const keptAfter = (from: number, upTo = output.length) => span(output.slice(from, upTo), from);
  return { join, asks, keptAfter, shown: () => shown };
}

describe("a terminal view's replay (SK-23)", () => {
  it("shows what was printed before it opened, and what comes after, once", () => {
    const out = "Welcome\r\nDo you want to proceed?\r\n❯ 1. Yes\r\n";
    const at = "Welcome\r\nDo you want to proceed?\r\n".length;
    const v = view(out);
    v.join.kept(v.keptAfter(0, at));
    v.join.heard(v.keptAfter(at));
    expect(v.shown()).toBe(out);
  });

  it("drops a chunk the kept output already holds, and shows only the new part of one that straddles it", () => {
    const out = "abcdefghij";
    const v = view(out);
    // Heard before the kept output came: held, then joined by place.
    v.join.heard(v.keptAfter(2, 6));
    v.join.heard(v.keptAfter(6, 8));
    v.join.kept(v.keptAfter(0, 4));
    v.join.heard(v.keptAfter(8));
    expect(v.shown()).toBe(out);
    expect(v.asks).toEqual([]);
  });

  it("asks for what came between the kept output and the first chunk, and shows it before what follows", () => {
    // The session printed its question after the kept output was read and
    // before the listener was in place, then waited (the 0.4.7 walk's N5).
    const out = "Welcome\r\nDo you want to proceed?\r\n";
    const v = view(out);
    v.join.kept(v.keptAfter(0, 9));
    v.join.listening();
    expect(v.asks).toEqual([9]);
    v.join.kept(v.keptAfter(9));
    expect(v.shown()).toBe(out);
  });

  it("asks once the live stream is in place, even before the kept output is back", () => {
    const out = "one\r\ntwo\r\n";
    const v = view(out);
    v.join.listening();
    expect(v.asks).toEqual([]);
    v.join.kept(v.keptAfter(0, 5));
    expect(v.asks).toEqual([5]);
    v.join.kept(v.keptAfter(5));
    expect(v.shown()).toBe(out);
  });

  it("fills a gap before a chunk, holding what follows until it's filled", () => {
    const out = "0123456789";
    const v = view(out);
    v.join.kept(v.keptAfter(0, 3));
    v.join.heard(v.keptAfter(6, 8));
    v.join.heard(v.keptAfter(8));
    expect(v.asks).toEqual([3]);
    expect(v.shown()).toBe("012");
    v.join.kept(v.keptAfter(3, 7));
    expect(v.shown()).toBe(out);
  });

  it("shows what's kept when the bytes between were let go, and goes on from there", () => {
    const v = view("x");
    v.join.kept(span("start", 0));
    v.join.heard(span("later", 100));
    expect(v.asks).toEqual([5]);
    // The tail kept now begins at 96: 5–95 are gone; "later" is in it.
    v.join.kept(span("taillater", 96));
    expect(v.shown()).toBe("starttaillater");
    // A failed ask shows what's heard from its own place.
    const w = view("y");
    w.join.kept(span("ab", 0));
    w.join.heard(span("zz", 10));
    w.join.lost();
    expect(w.shown()).toBe("abzz");
  });
});
