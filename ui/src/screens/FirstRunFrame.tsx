/**
 * First run's frame, moved out of FirstRun.tsx (its size budget): over
 * the whole window for Add apps, with its way out at the top while the
 * page scrolls (0.5.0, DLG-1); or (AJ-E1545) in place as the setup
 * wizard's Find your apps step, whose own header and foot carry the way
 * out — there only a bring-in under way shows its Stop here (ADR-0018 §5:
 * what finished stays).
 */
import type { JSX, ReactNode, RefObject } from "react";
import { Button } from "../components/primitives";

export function FirstRunFrame({
  embedded,
  mode,
  importing,
  stopping,
  leave,
  frameRef,
  children,
}: {
  embedded: boolean;
  mode: "first-run" | "add";
  importing: boolean;
  stopping: boolean;
  leave: () => void;
  frameRef: RefObject<HTMLDivElement | null>;
  children: ReactNode;
}): JSX.Element {
  const stop = (
    <Button size="sm" variant="secondary" onClick={leave} disabled={stopping} aria-keyshortcuts="Escape" data-testid="first-run-close">
      {stopping ? "Stopping after this one…" : importing ? "Stop" : mode === "add" ? "Close" : "Not now"}
    </Button>
  );
  if (embedded) {
    return (
      <div className="flex flex-col gap-4" data-testid="first-run" data-mode={mode} data-embedded="true">
        {importing && <div className="flex justify-end">{stop}</div>}
        {children}
      </div>
    );
  }
  return (
    <div
      className="fixed inset-0 z-50 flex flex-col overflow-y-auto bg-canvas text-ink outline-none"
      role="dialog"
      aria-modal="true"
      aria-label={mode === "add" ? "Add apps" : "Set up AI Juggler"}
      data-testid="first-run"
      data-mode={mode}
      ref={frameRef}
      tabIndex={-1}
    >
      {/* 0.5.0 (DLG-1): the way out stays at the top while the page scrolls. */}
      <div data-tauri-drag-region className="sticky top-0 z-20 flex h-11 shrink-0 items-center justify-end bg-canvas px-4" data-dialog-part="header">
        {/* Always a visible way out (0.0.7 dogfood: "no way to close"). */}
        {stop}
      </div>
      <div className="flex flex-1 justify-center px-6 pb-16 pt-6">
        <div className="flex w-full max-w-3xl flex-col gap-7">{children}</div>
      </div>
    </div>
  );
}
