/**
 * 0.4.6 (AJ-E3011 §3): who leads, never a dead end. D-92 stands: one
 * session leads. The picker offers, in this order:
 * - the sessions open now in the pool's projects, most recent first;
 * - "[App]'s next session", one row per App in the pool: its next session
 *   here leads (the pool's `lead_next`);
 * - "No lead for now", saying what a lead would add.
 * The setup's "Who leads" step and the canvas's inspector share it.
 */
import type { JSX } from "react";

import { ChoiceOption } from "../../../components/controls";
import type { LeadChoice } from "../../../lib/new-pool";
import { LEAD_WOULD, nextSessionWords } from "../../../lib/pool-words";

export interface LeadSession {
  ref: string;
  name: string;
  /** Its project, App and state. */
  detail: string;
}

export interface LeadApp {
  /** The App's member ref (`app:…`). */
  ref: string;
  name: string;
}

const same = (a: LeadChoice, b: LeadChoice) =>
  a.kind === b.kind && (a.kind === "none" || (a.kind === "session" && b.kind === "session" && a.ref === b.ref) || (a.kind === "next" && b.kind === "next" && a.app === b.app));

export function LeadPicker({
  sessions,
  apps,
  value,
  onChange,
  disabled = false,
  testId = "lead-picker",
}: {
  sessions: readonly LeadSession[];
  apps: readonly LeadApp[];
  value: LeadChoice;
  onChange: (choice: LeadChoice) => void;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  const row = (choice: LeadChoice, title: string, detail: string, key: string, kind: string) => (
    <ChoiceOption
      key={key}
      type="radio"
      name={`${testId}-choice`}
      checked={same(choice, value)}
      disabled={disabled}
      onChange={() => onChange(choice)}
      label={title}
      hint={detail}
      className={same(choice, value) ? "bg-surface-3" : ""}
      data-testid={`${testId}-option`}
      data-kind={kind}
      data-ref={key}
    />
  );
  return (
    <fieldset className="flex min-w-0 flex-col gap-3" data-testid={testId}>
      <legend className="sr-only">Who leads</legend>
      <div className="flex flex-col gap-1">
        <span className="section-label">Open now</span>
        {sessions.length === 0 ? (
          <p className="px-3 text-meta text-muted" data-testid={`${testId}-none-open`}>
            No session is open in these projects now.
          </p>
        ) : (
          <div className="flex flex-col gap-0.5">{sessions.map((session) => row({ kind: "session", ref: session.ref }, session.name, session.detail, session.ref, "session"))}</div>
        )}
      </div>
      {apps.length > 0 && (
        <div className="flex flex-col gap-1">
          <span className="section-label">Next session</span>
          <div className="flex flex-col gap-0.5">
            {apps.map((app) => row({ kind: "next", app: app.ref }, nextSessionWords(app.name), "Its next session in this pool leads. Until then the pool works without a lead.", app.ref, "next"))}
          </div>
        </div>
      )}
      <div className="flex flex-col gap-0.5">{row({ kind: "none" }, "No lead for now", LEAD_WOULD, "none", "none")}</div>
    </fieldset>
  );
}
