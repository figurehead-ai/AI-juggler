/**
 * E31 (AJ-E3103): "↑ N new" over a related list read while sessions
 * start. It floats over the rows and takes no room of its own (its height
 * and the column's gap cancel out), so the rows being read never move
 * when it comes (walk K2: they moved down 36 px); its click shows the
 * list in order and goes to its top, where the new rows are (walk K2: it
 * showed them 1 200 px above, out of sight).
 */
import { useRef, type JSX } from "react";

export function NewRowsBar({ waiting, onShow }: { waiting: string | null; onShow: () => void }): JSX.Element {
  const top = useRef<HTMLSpanElement>(null);
  return (
    <>
      <span ref={top} aria-hidden="true" className="-mb-2 block h-0" data-testid="related-top" />
      {waiting && (
        <div className="sticky top-1 z-20 -mb-2 flex h-0 justify-center overflow-visible" data-testid="related-waiting-bar">
          <button
            type="button"
            onClick={() => {
              onShow();
              top.current?.scrollIntoView?.({ block: "nearest" });
            }}
            className="h-fit rounded-full border border-accent/40 bg-accent-soft px-3 py-1 text-meta font-medium text-accent-hi shadow-card outline-none hover:bg-accent/15 focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="related-waiting"
          >
            <span aria-hidden="true">↑ </span>
            {waiting}
          </button>
        </div>
      )}
    </>
  );
}
