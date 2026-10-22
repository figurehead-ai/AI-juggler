/**
 * AJ-E1542/AJ-E1543: in the switch flow, what the target account lacks.
 * Each row says what it is and what happens: set up as it is, set up
 * translated, needs you (a login, a variable), or can't be set up there.
 * Rows that can be set up safely start ticked; the user can untick any,
 * or tick one that needs them. Nothing is written until the switch runs.
 */
import type { JSX } from "react";
import type { OfferInfo, SetupCheckInfo } from "../lib/ipc/switchSetup";

const KIND_WORDS: Record<string, string> = {
  "mcp-server": "MCP server",
  skill: "Skill",
  command: "Command",
  subagent: "Subagent",
  rules: "Rules",
  plugin: "Plugin",
};

function verdictWords(offer: OfferInfo): string {
  switch (offer.verdict) {
    case "copy":
      return "Set up as it is";
    case "translate":
      return `Set up ${offer.note ?? "in this tool's form"}`;
    case "needs-you":
      return `Needs you: ${offer.note ?? ""}`;
    default:
      return `Can't be set up: ${offer.note ?? ""}`;
  }
}

/** How many of the offers will be set up, in a sentence. */
export function setupSummary(check: SetupCheckInfo, picked: ReadonlySet<string>): string {
  const missing = check.offers.length;
  if (missing === 0) {
    return `${check.to} already has everything ${check.from} has set up.`;
  }
  const thing = missing === 1 ? "thing" : "things";
  return `${check.to} doesn't have ${missing} ${thing} ${check.from} has. ${picked.size} will be set up.`;
}

export function SwitchSetupList({
  check,
  picked,
  onToggle,
}: {
  check: SetupCheckInfo;
  picked: ReadonlySet<string>;
  onToggle: (id: string) => void;
}): JSX.Element {
  return (
    <section className="flex flex-col gap-2" data-testid="switch-setup" aria-labelledby="switch-setup-title">
      <h4 id="switch-setup-title" className="text-xs font-semibold">
        Set up on {check.to}
      </h4>
      <p className="text-xs text-muted" data-testid="switch-setup-summary">
        {setupSummary(check, picked)}
      </p>
      {check.offers.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {check.offers.map((offer) => {
            const settable = offer.verdict !== "impossible";
            const id = `switch-setup-${offer.id}`;
            return (
              <li key={offer.id} className="flex items-start gap-2 text-xs" data-testid="switch-setup-row">
                {settable ? (
                  <input
                    id={id}
                    type="checkbox"
                    className="mt-0.5"
                    checked={picked.has(offer.id)}
                    onChange={() => onToggle(offer.id)}
                  />
                ) : (
                  <span aria-hidden="true" className="mt-0.5 inline-block w-[13px]" />
                )}
                <label htmlFor={settable ? id : undefined} className={settable ? "" : "text-muted"}>
                  <span className="font-medium">{offer.name}</span>{" "}
                  <span className="text-muted">· {KIND_WORDS[offer.kind] ?? offer.kind}</span>
                  <span className="block text-muted">{verdictWords(offer)}</span>
                </label>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
