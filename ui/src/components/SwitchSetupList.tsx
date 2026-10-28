/**
 * AJ-E1542/AJ-E1543: in the switch flow, what the target account lacks.
 * What this conversation used comes first; what it didn't waits under one
 * collapsed line ("Also on Work Max, not on Personal"), and a `null`
 * `used` (the tool's transcripts don't say) is never read as "not used".
 * Each row says what happens and whether it holds up the switch: set up
 * as it is, translated, needs you (with the target tool's steps and
 * command to give a server its own secret, AJ-E1543), or, for what comes
 * with the other account (a claude.ai skill), how to turn it on there
 * (Show me how, its page, Check again). Rows that can be set up safely and were used start
 * ticked; nothing is written until the switch runs. Each choice has a
 * "What does this mean?" with an example and the switching guide.
 */
import { useState, type JSX } from "react";
import {
  switchSetupUndo,
  type AppliedInfo,
  type OfferInfo,
  type SetupCheckInfo,
} from "../lib/ipc/switchSetup";
import { Meaning, TurnOnSteps } from "./SwitchSetupTurnOn";
import { SecretSteps } from "./switch/SecretSteps";

const KIND_WORDS: Record<string, string> = {
  "mcp-server": "MCP server",
  skill: "Skill",
  command: "Command",
  subagent: "Subagent",
  rules: "Rules",
  "main-rules": "Main rules",
  plugin: "Plugin",
  permissions: "Permissions",
  "status-line": "Status line",
};

/** A backend note as the end of a sentence: with its full stop. */
function sentence(note: string | null): string {
  const text = note ?? "";
  return /[.!?]$/.test(text) ? text : `${text}.`;
}

/** What happens to it, and whether the switch waits for it, in a line. */
function consequence(offer: OfferInfo, check: SetupCheckInfo): string {
  switch (offer.verdict) {
    case "copy":
      return "Set up as it is when you switch. You can undo it after.";
    case "translate":
      return `Set up ${offer.note ?? "in this tool's form"} when you switch.`;
    case "needs-you":
      return `You can switch now, then finish it: ${sentence(offer.note)}`;
    case "turn-on":
      return `You can switch now. ${check.to} won't have ${offer.name} until you turn it on.`;
    default:
      return `You can switch now. ${check.to} won't have it: ${sentence(offer.note)}`;
  }
}

/** Where it came from, when that's why it can't simply be copied. */
function origin(offer: OfferInfo, check: SetupCheckInfo): string | null {
  switch (offer.origin) {
    case "provided":
      return `From ${offer.provided_by ?? "the tool's maker"}, with ${check.from}'s account`;
    case "account":
      return `From ${check.from}'s account: yours, your organization's or shared with you`;
    case "unknown":
      return `From ${check.from}'s account`;
    default:
      return null;
  }
}

/** "What does this mean?" for each verdict: a sentence and an example. */
function meaning(offer: OfferInfo, check: SetupCheckInfo): { text: string; example: string } {
  switch (offer.verdict) {
    case "copy":
      return {
        text: `AI Juggler copies it into ${check.to}'s own folder before the switch. Undo removes the copy while it's unchanged.`,
        example: "a skill folder is copied file by file; the original stays as it is.",
      };
    case "translate":
      return {
        text: "It's rewritten in the other tool's form, then set up there. Undo removes it.",
        example: "a Codex prompt becomes a Claude Code command.",
      };
    case "needs-you":
      return {
        text: "It can be set up, but one step is yours. Tick it to set it up; the switch doesn't wait for your step.",
        example: "a server that needs a token reads it from a variable you set yourself; secrets are never copied.",
      };
    case "turn-on":
      return offer.can_copy
        ? {
            text: `It came with ${check.from}'s account. It may be yours, your organization's or shared with you, and AI Juggler can't tell which. Turn it on for ${check.to}, or, if you made it, copy it.`,
            example: "a skill you wrote and uploaded can be copied; your organization's is turned on in the account's settings.",
          }
        : {
            text: `It comes with the account, so ${check.to} turns it on itself, and it arrives with ${check.to}'s next session. AI Juggler doesn't copy it: a copy wouldn't update, and the account decides who may have it.`,
            example: "a skill your plan includes is switched on in the account's settings; one your plan doesn't include can't be.",
          };
    default:
      return {
        text: `${check.to}'s tool has no way to use it, so it stays behind. The switch goes ahead without it.`,
        example: "a subagent from another tool that this one has no place for.",
      };
  }
}

/** How many of the offers will be set up, in a sentence. */
export function setupSummary(check: SetupCheckInfo, picked: ReadonlySet<string>): string {
  const missing = check.offers.length;
  if (missing === 0) {
    return `${check.to} already has everything ${check.from} has set up.`;
  }
  // Only a tool that says what was used can say "this conversation".
  const reported = check.offers.some((offer) => offer.used !== null);
  const lead = check.offers.filter((offer) => offer.used !== false);
  if (lead.length === 0) {
    const setUp = picked.size > 0 ? ` ${picked.size} will be set up.` : "";
    return `${check.to} has everything this conversation used.${setUp}`;
  }
  const thing = lead.length === 1 ? "thing" : "things";
  const what = reported ? "this conversation used" : `${check.from} has`;
  const turnOn = lead.filter((offer) => offer.verdict === "turn-on" && !picked.has(offer.id)).length;
  // "0 will be set up" says nothing when none of them can be.
  const settable = lead.some((offer) => ["copy", "translate", "needs-you"].includes(offer.verdict) || offer.can_copy);
  if (!settable) {
    const them = turnOn === lead.length ? (turnOn === 1 ? "it" : "them") : `${turnOn} of them`;
    return `${check.to} doesn't have ${lead.length} ${thing} ${what}.${turnOn > 0 ? ` You turn ${them} on yourself.` : ""}`;
  }
  const yours = turnOn > 0 ? `, ${turnOn} you turn on yourself` : "";
  return `${check.to} doesn't have ${lead.length} ${thing} ${what}. ${picked.size} will be set up${yours}.`;
}

function SetupRow({
  offer,
  check,
  picked,
  onToggle,
  onCheckAgain,
}: {
  offer: OfferInfo;
  check: SetupCheckInfo;
  picked: ReadonlySet<string>;
  onToggle: (id: string) => void;
  onCheckAgain?: (offerId: string) => Promise<string>;
}): JSX.Element {
  const settable = ["copy", "translate", "needs-you"].includes(offer.verdict);
  const id = `switch-setup-${offer.id}`;
  const kind = KIND_WORDS[offer.kind] ?? offer.kind;
  const from = origin(offer, check);
  const checkAgain = onCheckAgain && (() => onCheckAgain(offer.id));
  return (
    <li className="flex items-start gap-2" data-testid="switch-setup-row" data-verdict={offer.verdict}>
      {settable ? (
        <input
          id={id}
          type="checkbox"
          className="mt-1"
          checked={picked.has(offer.id)}
          onChange={() => onToggle(offer.id)}
        />
      ) : (
        <span aria-hidden="true" className="mt-1 inline-block w-[13px] shrink-0" />
      )}
      <div className="flex min-w-0 flex-col gap-1">
        <label htmlFor={settable ? id : undefined} className="text-body text-ink">
          {/* One of a kind (an account's permissions, its status line): its
              kind is its name, said once. */}
          {kind.toLowerCase() === offer.name.toLowerCase() ? (
            <span className="font-medium">{kind}</span>
          ) : (
            <>
              <span className="font-medium">{offer.name}</span> <span className="text-meta text-muted">· {kind}</span>
            </>
          )}
          {offer.used === true && (
            <span className="ml-2 rounded border border-hairline px-1.5 py-px text-label text-muted">Used here</span>
          )}
        </label>
        {offer.about && <span className="text-meta text-ink">{offer.about}</span>}
        {from && <span className="text-meta text-muted">{from}</span>}
        <span className="text-meta text-ink" data-testid="switch-setup-consequence">
          {consequence(offer, check)}
        </span>
        {offer.verdict === "turn-on" && offer.get_it && (
          <TurnOnSteps getIt={offer.get_it} name={offer.name} onCheckAgain={checkAgain} />
        )}
        {offer.verdict === "needs-you" && offer.get_it && (
          <SecretSteps getIt={offer.get_it} name={offer.name} to={check.to} onCheckAgain={checkAgain} />
        )}
        {offer.verdict === "turn-on" && offer.can_copy && (
          <label className="flex items-center gap-1.5 text-meta text-ink">
            <input type="checkbox" checked={picked.has(offer.id)} onChange={() => onToggle(offer.id)} />
            Copy {offer.name} to {check.to} now instead (only if you made it)
          </label>
        )}
        <Meaning {...meaning(offer, check)} />
      </div>
    </li>
  );
}

export function SwitchSetupList({
  check,
  picked,
  onToggle,
  onCheckAgain,
  bare = false,
}: {
  check: SetupCheckInfo;
  picked: ReadonlySet<string>;
  onToggle: (id: string) => void;
  /** Reads the target's set-up again (Check again, after turning
   * something on, or giving it its secret); resolves to what it found
   * for that offer, in words. */
  onCheckAgain?: (offerId: string) => Promise<string>;
  /** The rows only, under a line that already says what they are
   * (AJ-E1508's "Start with the same settings as …"). */
  bare?: boolean;
}): JSX.Element | null {
  if (check.offers.length === 0) {
    // Nothing missing: one line, and the switch screen stays short.
    if (bare || check.already_there === 0) return null;
    return (
      <p className="text-meta text-ink" data-testid="switch-setup-summary">
        {setupSummary(check, picked)}
      </p>
    );
  }
  const lead = check.offers.filter((offer) => offer.used !== false);
  const rest = check.offers.filter((offer) => offer.used === false);
  const row = (offer: OfferInfo) => (
    <SetupRow key={offer.id} offer={offer} check={check} picked={picked} onToggle={onToggle} onCheckAgain={onCheckAgain} />
  );
  return (
    <section
      className="flex flex-col gap-2"
      data-testid="switch-setup"
      aria-labelledby={bare ? undefined : "switch-setup-title"}
      aria-label={bare ? `What comes from ${check.from}` : undefined}
    >
      {!bare && (
        <>
          <h4 id="switch-setup-title" className="text-section text-ink">
            Set up on {check.to}
          </h4>
          <p className="text-meta text-ink" data-testid="switch-setup-summary">
            {setupSummary(check, picked)}
          </p>
        </>
      )}
      {lead.length > 0 && <ul className="flex flex-col gap-3">{lead.map(row)}</ul>}
      {rest.length > 0 && (
        <details className="flex flex-col gap-2" data-testid="switch-setup-unused">
          <summary className="cursor-pointer text-meta text-ink">
            Also on {check.from}, not on {check.to} ({rest.length}, not used in this conversation)
          </summary>
          <ul className="mt-2 flex flex-col gap-3">{rest.map(row)}</ul>
        </details>
      )}
    </section>
  );
}

/** What was set up when the switch ran, each with its own Undo. */
export function SwitchSetupResults({
  target,
  results,
}: {
  target: string;
  results: AppliedInfo[];
}): JSX.Element | null {
  const [undone, setUndone] = useState<Record<string, string>>({});
  if (results.length === 0) return null;
  const undo = async (record: string) => {
    try {
      const message = await switchSetupUndo(record);
      setUndone((current) => ({ ...current, [record]: message }));
    } catch (cause) {
      setUndone((current) => ({ ...current, [record]: String(cause) }));
    }
  };
  return (
    <section className="flex flex-col gap-1" data-testid="switch-setup-results">
      <h4 className="text-section font-semibold text-ink">Set up on {target}</h4>
      <ul className="flex flex-col gap-1">
        {results.map((result) => {
          const record = result.record;
          return (
            <li key={result.offer} className={"flex items-center gap-2 text-label " + (record ? "text-muted" : "text-warning")}>
              <span>{record && undone[record] ? undone[record] : result.message}</span>
              {record && !undone[record] && (
                <button
                  type="button"
                  onClick={() => void undo(record)}
                  aria-label={`Undo: ${result.message}`}
                  className="rounded border border-hairline px-1.5 py-0.5 text-label hover:bg-surface-2"
                >
                  Undo
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
