/**
 * 0.4.6 (D-126): who leads, as the kit's choice cards with each one's
 * logo — the same choices, lines and read-back as the diagram's lead
 * control (`LeadPicker`, AJ-E3011 §3; paths-parity holds them word for
 * word), never a dead end. D-92: one session leads. In order: the
 * sessions open now in the pool's projects, each App's next session
 * there, and No lead for now; then the choice read back.
 */
import type { JSX } from "react";

import { RadioCard, RadioCards } from "../../components/controls";
import { IconRemove } from "../../components/icons-actions";
import { LogoTile } from "../../components/ToolLogo";
import type { LeadChoice } from "../../lib/new-pool";
import { LEAD_WOULD, leadSentence, nextSessionWords } from "../../lib/pool-words";
import type { LeadApp, LeadSession } from "../../pages/pool/setup/LeadPicker";
import { logoOf, MemberLogo, type LogoSources } from "../../components/MemberLogo";

/** A choice as its card's value: `session|<ref>`, `next|<app>`, `none`. */
const leadValue = (choice: LeadChoice): string => (choice.kind === "none" ? "none" : choice.kind === "session" ? `session|${choice.ref}` : `next|${choice.app}`);

function choiceOf(value: string): LeadChoice {
  const [kind, ref = ""] = value.split("|");
  return kind === "session" ? { kind: "session", ref } : kind === "next" ? { kind: "next", app: ref } : { kind: "none" };
}

export function LeadCards({
  sessions,
  apps,
  value,
  onChange,
  where = "these projects",
  logos,
  testId = "lead-cards",
}: {
  sessions: readonly LeadSession[];
  apps: readonly LeadApp[];
  value: LeadChoice;
  onChange: (choice: LeadChoice) => void;
  /** The pool's projects in words ("web"). */
  where?: string;
  /** Where each choice's logo is looked up. */
  logos: LogoSources;
  testId?: string;
}): JSX.Element {
  const chosenSession = value.kind === "session" ? sessions.find((one) => one.ref === value.ref) : undefined;
  const chosenApp = value.kind === "next" ? apps.find((app) => app.ref === value.app) : undefined;
  const label = (words: string) => <p className="section-label mt-1.5 first:mt-0 sm:col-span-2">{words}</p>;
  return (
    <fieldset className="flex min-w-0 flex-col gap-3.5" data-testid={testId}>
      <legend className="sr-only">Who leads</legend>
      <RadioCards label="Who leads" value={leadValue(value)} onChange={(next) => onChange(choiceOf(next))} columns={2}>
        {label("Open now")}
        {sessions.length === 0 ? (
          <p className="text-meta text-muted sm:col-span-2" data-testid={`${testId}-none-open`}>
            No session is open in these projects now.
          </p>
        ) : (
          sessions.map((session) => (
            <RadioCard
              key={session.ref}
              value={leadValue({ kind: "session", ref: session.ref })}
              logo={<MemberLogo spec={logoOf(session.ref, logos)} size={40} />}
              title={session.name}
              sub={session.detail}
              testId={`${testId}-option`}
            />
          ))
        )}
        {apps.length > 0 && label("Next session")}
        {apps.map((app) => (
          <RadioCard
            key={app.ref}
            value={leadValue({ kind: "next", app: app.ref })}
            logo={<MemberLogo spec={logoOf(app.ref, logos)} size={40} />}
            title={nextSessionWords(app.name)}
            sub={app.note ?? `It takes charge as soon as one of its sessions starts in ${where}.`}
            disabled={!!app.blocked}
            reason={app.blocked ?? undefined}
            testId={`${testId}-option`}
          />
        ))}
        <RadioCard value="none" logo={<LogoTile icon={IconRemove} size={40} />} title="No lead for now" sub={LEAD_WOULD} testId={`${testId}-option`} />
      </RadioCards>
      {/* Round 3: the read-back is the control's own, in both paths. */}
      <p className="text-meta text-ink" data-testid={`${testId}-sentence`}>
        {leadSentence(chosenSession?.name ?? null, chosenApp?.name ?? null, where)}
      </p>
    </fieldset>
  );
}
