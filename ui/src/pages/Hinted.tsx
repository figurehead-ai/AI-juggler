/**
 * An action with a one-line hint under it, in the hint's style; laid out
 * of the row's flow, so the buttons beside it keep their line (a page
 * header's Relaunch "Fresh conversation", Resume "Same conversation").
 */
import type { JSX, ReactNode } from "react";

export function Hinted({ hint, testId, children }: { hint: string | null; testId: string; children: ReactNode }): JSX.Element {
  if (hint === null) return <>{children}</>;
  // The line wraps to its button's width and takes no height: side by
  // side, two never print over each other, and the row gets no wider (a
  // wider one cut the App chip to "Claude Code · Work …" at 1280).
  return (
    <span className="flex flex-col items-center">
      {children}
      <span className="pointer-events-none h-0 w-0 min-w-full">
        <span className="relative top-1 block text-center text-label leading-tight text-muted" data-testid={testId}>
          {hint}
        </span>
      </span>
    </span>
  );
}
