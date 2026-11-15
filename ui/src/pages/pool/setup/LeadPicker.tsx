/**
 * 0.4.6 (AJ-E3011 §3): who leads, never a dead end. D-92 stands: one
 * session leads. The picker offers, in this order:
 * - the sessions open now in the pool's projects, most recent first;
 * - "[App]'s next session", one row per App in the pool: its next session
 *   here leads (the pool's `lead_next`);
 * - "No lead for now", saying what a lead would add.
 * The setup's "Who leads" step and the canvas's inspector share it.
 * Round 2 (#3): an App's row says it leads from its next session in the
 * pool's projects; on a made pool, "Start one now" opens New session on
 * that App there (the user starts it).
 */
import type { JSX } from "react";

import { ChoiceOption } from "../../../components/controls";
import { SELECTED_FILL } from "../../../components/controls/selected";
import type { LeadChoice } from "../../../lib/new-pool";
import { LEAD_WOULD, leadSentence, nextSessionWords } from "../../../lib/pool-words";
import { StartLeadButton } from "./StartLead";

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
  /** A session of it can start here (a CLI App, the pool made). */
  startable?: boolean;
  /** Its sessions open in the pool's projects now, when known. */
  open?: number;
}

const same = (a: LeadChoice, b: LeadChoice) =>
  a.kind === b.kind && (a.kind === "none" || (a.kind === "session" && b.kind === "session" && a.ref === b.ref) || (a.kind === "next" && b.kind === "next" && a.app === b.app));

export function LeadPicker({
  sessions,
  apps,
  value,
  onChange,
  disabled = false,
  where = "these projects",
  onStart,
  testId = "lead-picker",
}: {
  sessions: readonly LeadSession[];
  apps: readonly LeadApp[];
  value: LeadChoice;
  onChange: (choice: LeadChoice) => void;
  disabled?: boolean;
  /** The pool's projects in words ("web"). */
  where?: string;
  /** A made pool: starts the chosen App's session that will lead. */
  onStart?: (app: string) => void;
  testId?: string;
}): JSX.Element {
  const starting = value.kind === "next" && onStart ? apps.find((app) => app.ref === value.app && app.startable) : undefined;
  const chosenSession = value.kind === "session" ? sessions.find((one) => one.ref === value.ref) : undefined;
  const chosenApp = value.kind === "next" ? apps.find((app) => app.ref === value.app) : undefined;
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
      className={same(choice, value) ? SELECTED_FILL : ""}
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
            {apps.map((app) =>
              row({ kind: "next", app: app.ref }, nextSessionWords(app.name), `It leads from its next session in ${where}.`, app.ref, "next"),
            )}
          </div>
          {starting && onStart && (
            <span className="flex flex-wrap items-center gap-2 px-3 pt-1 text-meta text-ink-2" data-made-pool-only>
              <StartLeadButton member={starting.ref} name={starting.name} where={where} onStart={onStart} disabled={disabled} testId={`${testId}-start`} />
              New session on {starting.name} in {where}: it leads once you start it.
            </span>
          )}
        </div>
      )}
      <div className="flex flex-col gap-0.5">{row({ kind: "none" }, "No lead for now", LEAD_WOULD, "none", "none")}</div>
      {/* Round 3: the read-back is the control's own, in both paths. */}
      <p className="text-meta text-ink" data-testid={`${testId}-sentence`}>
        {leadSentence(chosenSession?.name ?? null, chosenApp?.name ?? null, where, chosenApp?.open)}
      </p>
    </fieldset>
  );
}
