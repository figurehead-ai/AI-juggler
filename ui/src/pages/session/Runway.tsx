/**
 * AJ-E2108 — an account's runway on a session's page: each window's name
 * and what's left on one line, its bar under it, then when it resets. The
 * words and the rule are AccountRunway's (`runwayLines`): one runway
 * everywhere, here at the session page's sizes.
 */
import type { JSX, ReactNode } from "react";
import { Card } from "../../components/primitives";
import { runwayLines } from "../../components/AccountRunway";
import type { QuotaDisplayData } from "../../lib/bridge";
import { CardTitle } from "./CardTitle";

const BAR = { ok: "bg-accent", low: "bg-warning", out: "bg-danger", none: "" } as const;
const WORDS = { ok: "text-ink/80", low: "text-warning", out: "text-danger", none: "text-ink/80" } as const;

export function Runway({ display, title, heading = "Runway", testId = "session-runway", children }: { display: QuotaDisplayData | undefined; title: string | null; heading?: string; testId?: string; children?: ReactNode }): JSX.Element {
  const lines = display?.unavailable_reason ? [] : runwayLines(display);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid={testId}>
      <div className="flex items-baseline justify-between gap-3">
        <CardTitle>{heading}</CardTitle>
        {title && <span className="min-w-0 truncate text-[13px] text-ink/80">{title}</span>}
      </div>
      {display?.unavailable_reason ? (
        <p className="text-[13px] text-ink/80" data-testid="account-runway">
          {display.unavailable_reason}
        </p>
      ) : (
        <div className="flex flex-col gap-3" data-testid="account-runway">
          {lines.map((line) => (
            <div key={line.label} className="flex flex-col gap-1.5">
              <div className="flex items-baseline justify-between gap-3 text-[13px]">
                <span className="text-ink/80">{line.label}</span>
                {line.left !== null && <span className="font-semibold tabular-nums text-ink">{line.left}% left</span>}
              </div>
              <span aria-hidden="true" className={"h-1.5 overflow-hidden rounded-full " + (line.left === null ? "border border-dashed border-hairline" : "bg-surface-3")}>
                <span className={"block h-full rounded-full " + BAR[line.tone]} style={{ width: `${line.left ?? 0}%` }} />
              </span>
              <span className={"text-[12px] " + WORDS[line.tone]}>{line.words.replace(/^\d+% left(?: · )?/, "")}</span>
            </div>
          ))}
        </div>
      )}
      {children}
    </Card>
  );
}
