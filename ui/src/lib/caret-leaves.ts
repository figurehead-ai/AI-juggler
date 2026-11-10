/**
 * rc24 walk K1: after a session's page, every page showed macOS's
 * input-source mark (a small ⊖) at the bottom-left of the content area.
 * macOS draws it at the focused field's caret, and the terminal's field
 * still had focus when its page went: in WebKit a click on a button takes
 * no focus, so the field was removed without ever being blurred, and the
 * mark stayed where its caret had been, over whatever page came next.
 *
 * A part holding a text field blurs it as it goes, while it is still on
 * the page: the caret, and the mark, go with it.
 */
import { useLayoutEffect, type DependencyList, type RefObject } from "react";

export function useCaretLeavesWith(ref: RefObject<HTMLElement | null>, deps: DependencyList = []): void {
  useLayoutEffect(() => {
    const node = ref.current;
    return () => {
      const active = document.activeElement;
      // A layout effect's cleanup runs before React takes the node out.
      if (node && active instanceof HTMLElement && node.contains(active)) active.blur();
    };
    // The node to watch changes only with what the caller names.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}
