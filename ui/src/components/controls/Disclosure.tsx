/**
 * 0.3 — the app's disclosure, in place of the browser's ▸ triangle: a
 * button with the rail's fold chevron (turned when open), a focus ring,
 * and the same quiet type everywhere.
 *
 * Round 7 (AJ-E3011): a button with `aria-expanded`, not `<details>`: a
 * `<summary>` laid out as flex isn't a disclosure to WebKit's
 * accessibility, so an accessibility press did nothing (the aij-test
 * driver pressed "Show the files" four times). What it folds stays in
 * the page while closed, hidden, as `<details>` kept it.
 */
import { useId, useState, type JSX, type ReactNode } from "react";
import { FoldChevron } from "./FoldChevron";

const TONES = {
  quiet: "text-meta text-muted hover:text-ink",
  heading: "text-meta font-semibold text-ink",
  title: "text-body font-semibold text-ink",
  // A link's look, for a fold inside a line of text (a file's text).
  link: "text-meta text-accent-hi hover:underline",
} as const;

export function Disclosure({
  summary,
  children,
  open,
  onToggle,
  tone = "quiet",
  testId,
  className = "",
  summaryClassName = "",
}: {
  summary: ReactNode;
  children: ReactNode;
  /** Open at first; with `onToggle`, as its owner keeps it. */
  open?: boolean;
  onToggle?: (open: boolean) => void;
  /** A quiet line ("Details"), a group's heading in a card (Settings'
   * Terminal), a section's title (a dialog's Details), or a link. */
  tone?: keyof typeof TONES;
  testId?: string;
  className?: string;
  summaryClassName?: string;
}): JSX.Element {
  const [own, setOwn] = useState(open ?? false);
  // Kept by its owner when it says so (open and onToggle both given).
  const shown = onToggle && open !== undefined ? open : own;
  const body = useId();
  const toggle = () => {
    setOwn(!shown);
    onToggle?.(!shown);
  };
  return (
    <div data-testid={testId} data-disclosure data-open={shown || undefined} className={className}>
      <button
        type="button"
        aria-expanded={shown}
        aria-controls={body}
        onClick={toggle}
        className={"flex w-fit min-h-6 cursor-pointer items-center gap-1.5 rounded-md text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + TONES[tone] + " " + summaryClassName}
        data-testid={testId ? `${testId}-toggle` : undefined}
      >
        <FoldChevron open={shown} />
        {summary}
      </button>
      <div id={body} hidden={!shown}>
        {children}
      </div>
    </div>
  );
}
