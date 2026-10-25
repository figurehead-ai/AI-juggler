/**
 * AJ-E2108 — an account's runway on a session's page: each window's name
 * and what's left on one line, its bar under it, then when it resets. The
 * words and the rule are AccountRunway's (`runwayLines`): one runway
 * everywhere, here at the session page's sizes. Neither window measured:
 * one line that says so and what would measure it (the review of 0.1.5:
 * "Not measured yet" twice), never two empty bars.
 */
import type { JSX, ReactNode } from "react";
import { Card } from "../../components/primitives";
import { runwayLines } from "../../components/AccountRunway";
import { useActionHost } from "../../components/ItemMenu";
import type { AccountInfo, QuotaDisplayData } from "../../lib/bridge";
import { CardTitle } from "./CardTitle";

const BAR = { ok: "bg-accent", low: "bg-warning", out: "bg-danger", none: "" } as const;
const WORDS = { ok: "text-muted", low: "text-warning", out: "text-danger", none: "text-muted" } as const;

export function Runway({
  display,
  title,
  heading = "Runway",
  testId = "session-runway",
  profile = null,
  children,
}: {
  display: QuotaDisplayData | undefined;
  title: string | null;
  heading?: string;
  testId?: string;
  /** Its profile: where "Measure its limits" goes while nothing is measured. */
  profile?: AccountInfo | null;
  children?: ReactNode;
}): JSX.Element {
  const host = useActionHost();
  const lines = display?.unavailable_reason ? [] : runwayLines(display);
  const unmeasured = lines.length > 1 && lines.every((line) => line.left === null && line.words === lines[0]!.words);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid={testId}>
      <div className="flex items-baseline justify-between gap-3">
        <CardTitle>{heading}</CardTitle>
        {title && <span className="min-w-0 truncate text-meta text-muted">{title}</span>}
      </div>
      {display?.unavailable_reason ? (
        <p className="text-meta text-muted" data-testid="account-runway">
          {display.unavailable_reason}
        </p>
      ) : unmeasured ? (
        <div className="flex flex-col gap-1.5" data-testid="account-runway" data-state="not-measured">
          <p className="text-meta text-ink">{lines[0]!.words}</p>
          <p className="text-label leading-[1.5] text-muted">
            Its 5-hour and weekly windows show here once they're read.{" "}
            {profile && host.openAccount && (
              <button
                type="button"
                onClick={() => host.openAccount?.(profile)}
                className="rounded font-medium text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
                data-testid="runway-measure"
              >
                Measure its limits ›
              </button>
            )}
          </p>
        </div>
      ) : (
        <div className="flex flex-col gap-3" data-testid="account-runway">
          {lines.map((line) => (
            <div key={line.label} className="flex flex-col gap-1.5">
              <div className="flex items-baseline justify-between gap-3 text-meta">
                <span className="text-muted">{line.label}</span>
                {line.left !== null && <span className="font-semibold tabular-nums text-ink">{line.left}% left</span>}
              </div>
              <span aria-hidden="true" className={"h-1.5 overflow-hidden rounded-full " + (line.left === null ? "border border-dashed border-hairline" : "bg-surface-3")}>
                <span className={"block h-full rounded-full " + BAR[line.tone]} style={{ width: `${line.left ?? 0}%` }} />
              </span>
              <span className={"text-label " + WORDS[line.tone]}>{line.words.replace(/^\d+% left(?: · )?/, "")}</span>
            </div>
          ))}
        </div>
      )}
      {children}
    </Card>
  );
}
