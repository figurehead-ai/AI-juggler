/**
 * The "New messages" pill a conversation shows to a reader who scrolled
 * up while words came (`useStickToBottom`): a click takes them down.
 */
import type { JSX } from "react";

export function NewMessagesPill({ onClick, testId, className = "bottom-3" }: { onClick: () => void; testId: string; className?: string }): JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      className={
        "absolute left-1/2 inline-flex -translate-x-1/2 items-center gap-1.5 rounded-full border border-hairline bg-surface px-3.5 py-1.5 text-meta font-medium text-ink shadow-md outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60 " +
        className
      }
      data-testid={testId}
    >
      New messages
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 5v14M6 13l6 6 6-6" />
      </svg>
    </button>
  );
}
