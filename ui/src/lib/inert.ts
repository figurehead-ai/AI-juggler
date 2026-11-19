/**
 * AJ-E3011 round 4 (main's ruling, app-wide): while a menu or popover is
 * open, everything else on the page is inert — out of the accessibility
 * tree and the pointer's reach — so a press meant for one of its options
 * can never land on what's underneath (in the Dev window an accessibility
 * press on "Its role" › Reviewer reached "Take out of the pool" under it).
 *
 * Every sibling of the open element and of each of its ancestors, up to
 * the body, is made inert; nested menus and popovers share it by count,
 * so closing one leaves the other's in place. A press outside still
 * reaches the window (the menus' outside-press closes them), never the
 * control under it.
 */
import { useEffect, useLayoutEffect, type RefObject } from "react";

/** How many open menus and popovers made each element inert. */
const held = new Map<Element, number>();

function hold(element: Element): void {
  const count = held.get(element) ?? 0;
  if (count === 0) {
    // Inert already by its own owner (a closed panel): not ours to undo.
    if (element.hasAttribute("inert")) return;
    element.setAttribute("inert", "");
  }
  held.set(element, count + 1);
}

function release(element: Element): void {
  const count = held.get(element);
  if (count === undefined) return;
  if (count > 1) held.set(element, count - 1);
  else {
    held.delete(element);
    element.removeAttribute("inert");
  }
}

/** The elements beside `open` and beside each of its ancestors. */
export function besides(open: Element): Element[] {
  const out: Element[] = [];
  for (let at: Element | null = open; at && at !== document.body && at.parentElement; at = at.parentElement) {
    for (const sibling of at.parentElement.children) {
      // Scripts, styles and a live region announcing a change stay as they are.
      if (sibling === at || sibling.tagName === "SCRIPT" || sibling.tagName === "STYLE" || sibling.getAttribute("aria-live")) continue;
      out.push(sibling);
    }
  }
  return out;
}

/** Everything on the page but `kept` and what holds them: the elements
 * beside each, and beside each of their ancestors, that hold none. */
function besidesAll(kept: readonly Element[]): Element[] {
  const out = new Set<Element>();
  for (const one of kept) for (const other of besides(one)) out.add(other);
  return [...out].filter((other) => !kept.some((one) => other.contains(one)));
}

/** Makes everything but `ref`'s element inert while `on`; `keep`, the
 * control that opened it, stays live too, so pressing it again closes
 * it (F-rc3g-6, the 0.4.6 rc3g walk: with the bell's panel open, a
 * press on the bell found "nothing to press there").
 *
 * Its opener (what had focus as it opened) gets focus back when it closes
 * if focus was left nowhere: a close that hands focus back in the same
 * turn (Escape: `setOpen(false); button.focus()`) does so while the
 * opener is still inert, and an inert element takes no focus, so focus
 * fell to the page (0.4.6 RC, the legend's "?" and six more). */
export function useInertOutside(ref: RefObject<HTMLElement | null>, on = true, keep?: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const open = ref.current;
    const kept = keep?.current;
    const others = on && open ? besidesAll(kept ? [open, kept] : [open]) : [];
    const focused = document.activeElement;
    const opener = others.length > 0 && focused instanceof HTMLElement && focused !== document.body && !open?.contains(focused) ? focused : null;
    for (const one of others) hold(one);
    return () => {
      for (const one of others) release(one);
      const now = document.activeElement;
      if (opener?.isConnected && (now === null || now === document.body || (open?.contains(now) ?? false))) opener.focus({ preventScroll: true });
    };
  }, [ref, on, keep]);
}

/** The topmost modal dialog shown now: the last `[aria-modal="true"]` in
 * the page's order that isn't hidden, nor inside a part inert by its own
 * owner (a closed panel). One inside a part these holds made inert counts:
 * a dialog opened over another (the busy-folder offer over New session)
 * is the top one then. */
function topModal(): HTMLElement | null {
  const shownNow = (one: HTMLElement) => {
    if (one.closest("[hidden]")) return false;
    const inertAbove = one.closest("[inert]");
    return inertAbove === null || held.has(inertAbove);
  };
  return [...document.querySelectorAll<HTMLElement>('[aria-modal="true"]')].filter(shownNow).at(-1) ?? null;
}

/** The 0.4.6 rc2 walk (A8): a dialog only dimmed the window under it. An
 * accessibility press (VoiceOver's, the walk's) on the New session sheet
 * reached Home's "Start" behind it and opened a Claude app. Mounted once at
 * the app's root: while any modal dialog shows (`aria-modal="true"`, the 37
 * the app has, whoever draws them), everything outside the top one is
 * inert, by the same counts the menus use; when it goes, its opener gets
 * focus back if focus was left on the page (its own close hands it back
 * while the opener is still inert, as the menus' did). */
export function useModalsInert(): void {
  useEffect(() => {
    let shown: HTMLElement | null = null;
    let others: Element[] = [];
    // The page's control that opened the first dialog of a stack.
    let opener: HTMLElement | null = null;
    // What last had focus outside every dialog: the opener, even when a
    // dialog's own effect moved focus inside before this saw it.
    const active = document.activeElement;
    let outside: HTMLElement | null = active instanceof HTMLElement && active !== document.body ? active : null;
    // A dialog's own frame counts as the dialog: some focus the overlay
    // that holds their aria-modal box (the rc3 gate: the Stop… sheet focused
    // its root, which was taken for the opener).
    const focused = (event: FocusEvent) => {
      const at = event.target;
      if (at instanceof HTMLElement && !at.closest('[aria-modal="true"]') && !at.querySelector('[aria-modal="true"]')) outside = at;
    };
    // Focus left nowhere: on the page, or still on an element the closed
    // dialog took with it (the browser moves it to the page only at its
    // next drawing, after this has run: the rc3 gate's Stop… sheet).
    const lost = (gone: HTMLElement | null = null) => {
      const now = document.activeElement;
      return now === null || now === document.body || !now.isConnected || (gone?.contains(now) ?? false);
    };
    const update = () => {
      const top = topModal();
      if (top === shown) return;
      const was = shown;
      for (const one of others) release(one);
      others = [];
      shown = top;
      if (top) {
        opener ??= outside && !top.contains(outside) ? outside : null;
        others = besides(top).filter((one) => !one.contains(top));
        for (const one of others) hold(one);
        // A dialog closed over this one left focus nowhere: back inside.
        if (lost(was)) top.querySelector<HTMLElement>(FOCUSABLE)?.focus({ preventScroll: true });
        return;
      }
      if (lost(was)) again(opener)?.focus({ preventScroll: true });
      opener = null;
    };
    let queued = false;
    const watch = new MutationObserver(() => {
      if (queued) return;
      queued = true;
      queueMicrotask(() => {
        queued = false;
        update();
      });
    });
    document.addEventListener("focusin", focused);
    watch.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["aria-modal", "hidden"] });
    update();
    return () => {
      watch.disconnect();
      document.removeEventListener("focusin", focused);
      for (const one of others) release(one);
    };
  }, []);
}

/** The opener, or the element that took its place: a dialog's own close
 * hands focus to its trigger while the page is still inert (that fails),
 * and a list may have drawn the row again meanwhile (the 0.4.6 rc3 gate:
 * Processes' Stop… sheet, Esc, focus on the page); found by its test id. */
function again(opener: HTMLElement | null): HTMLElement | null {
  if (!opener) return null;
  if (opener.isConnected) return opener;
  const key = opener.getAttribute("data-testid");
  return key ? document.querySelector<HTMLElement>(`[data-testid="${CSS.escape(key)}"]`) : null;
}

/** What takes focus in a dialog. */
const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Tests: whatever is held is let go. */
export function releaseAllInert(): void {
  for (const element of held.keys()) element.removeAttribute("inert");
  held.clear();
}
