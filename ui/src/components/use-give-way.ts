/**
 * D-100 in a row of chips: its secondary words give way in steps, whole,
 * so the names give way last. 0.4.4: P3 put the day in an ended
 * session's state chip ("Stopped with an error yesterday 10:26 PM") and
 * the App chip beside it was cut to "· W" at 1280 px.
 *
 * A part that gives way says from which step it's left out
 * (`data-give-at="2"`); a shorter stand-in, from which step it shows
 * (`data-give-from="1"`). The row takes the first step at which all it
 * holds fits unshrunk, again whenever its width or its words change, or
 * a font lands. A part left out carries `data-gone`, its own classes
 * saying how (hidden; or read, not seen). What still doesn't fit at the
 * last step shrinks as its classes say: the name last. The room it sits
 * in, when marked, hears what it needs at its last step (`tellRoom`).
 */
import { useLayoutEffect, type RefObject } from "react";

const PARTS = "[data-give-at],[data-give-from]";

/** Whether `part` is left out at `step`. */
function goneAt(part: HTMLElement, step: number): boolean {
  const from = Number(part.dataset.giveFrom ?? 0);
  const at = part.dataset.giveAt === undefined ? Number.POSITIVE_INFINITY : Number(part.dataset.giveAt);
  return step < from || step >= at;
}

function take(parts: readonly HTMLElement[], step: number): void {
  for (const part of parts) part.toggleAttribute("data-gone", goneAt(part, step));
}

/** What `row` needs with nothing in it shrunk. */
function unshrunk(row: HTMLElement): number {
  row.style.width = "max-content";
  row.style.maxWidth = "none";
  const width = row.getBoundingClientRect().width;
  row.style.width = "";
  row.style.maxWidth = "";
  return width;
}

/** Takes the first step at which `row` fits, else its last. Its room is
 * measured with nothing left out: a row sized by what it holds (a folded
 * header's chips) is measured at its widest. */
function giveWay(row: HTMLElement): void {
  const parts = Array.from(row.querySelectorAll<HTMLElement>(PARTS));
  const last = Math.max(0, ...parts.map((part) => Number(part.dataset.giveAt ?? part.dataset.giveFrom ?? 0)));
  let step = 0;
  take(parts, step);
  const room = row.getBoundingClientRect().width;
  let need = unshrunk(row);
  // A hundredth of a pixel: what layout rounds, never a cut letter.
  while (step < last && need > room + 0.01) {
    step += 1;
    take(parts, step);
    need = unshrunk(row);
  }
  tellRoom(row, parts, step, last, need);
}

/** Says what `row` needs with all it can leave out left out to the room
 * it sits in (`data-give-way-room`, as `--give-way-need`): a page header
 * moves its actions down a line before a name is cut (D-100, 0.4.4: the
 * App chip read nothing at 1024 px beside an ended session's four
 * actions). Not its room's own width: the same at every width, so the
 * room never sways between a line and two. */
function tellRoom(row: HTMLElement, parts: readonly HTMLElement[], step: number, last: number, need: number): void {
  const room = row.closest<HTMLElement>("[data-give-way-room]");
  if (!room) return;
  let atLast = need;
  if (step < last) {
    take(parts, last);
    atLast = unshrunk(row);
    take(parts, step);
  }
  const words = `${Math.ceil(atLast)}px`;
  if (room.style.getPropertyValue("--give-way-need") !== words) room.style.setProperty("--give-way-need", words);
}

/** Keeps the row `inRow` sits in (its parent: the chips' line) at its
 * step (`giveWay`). */
export function useGiveWay(inRow: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const row = inRow.current?.parentElement;
    if (!row) return undefined;
    const fit = () => giveWay(row);
    fit();
    // Its words change (the time's day, the App's name once read): not
    // what this changes itself, which is attributes only.
    const words = new MutationObserver(fit);
    words.observe(row, { childList: true, characterData: true, subtree: true });
    const width = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(fit);
    width?.observe(row);
    const fonts = typeof document === "undefined" ? undefined : document.fonts;
    fonts?.addEventListener?.("loadingdone", fit);
    return () => {
      words.disconnect();
      width?.disconnect();
      fonts?.removeEventListener?.("loadingdone", fit);
    };
  }, [inRow]);
}
