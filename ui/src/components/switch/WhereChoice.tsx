/**
 * 0.3.5 — where the switched conversation continues, always two choices
 * (the maintainer, 2026-11-02: "always give two options … original
 * source Desktop Account or new Terminal Account … just call it a CLI and
 * Desktop"): Desktop, the target account's Claude app profile, and CLI,
 * the tool in a terminal. Where the original ran is chosen at first and
 * marked "Same as the original". Desktop is never hidden: when the target
 * has no Claude app profile it shows greyed, with why. The last button
 * names the choice ("Switch to Desktop").
 */
import type { JSX } from "react";
import type { PreflightInfoData, SwitchExecutedData } from "../../lib/ipc/switch-preflight";
import { parseSourceKey } from "../../lib/ipc/conversations";
import { markOpening } from "../../lib/just-opened";
import type { DestinationsInfo } from "../../types/generated/DestinationsInfo";
import { toolSpec } from "../../lib/tools";
import { RadioCard, RadioCards } from "../controls";
import { Pill } from "../Pill";
import { Section } from "../Section";
import { ToolLogo } from "../ToolLogo";

export type Destination = "desktop" | "cli";

/** What the preflight says; a backend that predates 0.3.5 offers CLI. */
export function destinationsOf(preflight: Pick<PreflightInfoData, "destinations">): DestinationsInfo {
  const info = preflight.destinations;
  if (!info) return { suggested: "cli", desktop: null, desktop_why_not: null, cli_why_not: null };
  // A backend before 0.4 sends no cli_why_not.
  return { ...info, cli_why_not: info.cli_why_not ?? null };
}

/** Where it ran, when that can be picked; else CLI (Desktop when CLI
 * can't be picked: a Claude app profile signed in through the app). */
export function defaultDestination(info: DestinationsInfo): Destination {
  if (info.desktop !== null && (info.suggested === "desktop" || info.cli_why_not)) return "desktop";
  return "cli";
}

/** The user-facing name of a destination. */
export function destinationName(destination: Destination): string {
  return destination === "desktop" ? "Desktop" : "CLI";
}

/** The last button: where it goes ("Switch to CLI anyway" when the
 * target is out of room or low). */
export function switchLabel(destination: Destination, risky: boolean): string {
  return `Switch to ${destinationName(destination)}${risky ? " anyway" : ""}`;
}

export function WhereChoice({
  preflight,
  value,
  onChange,
}: {
  preflight: PreflightInfoData;
  value: Destination;
  onChange: (destination: Destination) => void;
}): JSX.Element {
  const info = destinationsOf(preflight);
  const spec = toolSpec(preflight.target_tool);
  const desktop = info.desktop;
  const same = (destination: Destination) => info.suggested === destination && (destination === "cli" || desktop !== null);
  const title = (word: string, destination: Destination) => (
    <span className="inline-flex flex-wrap items-center gap-x-2 gap-y-1">
      {word}
      {same(destination) && <Pill tone="neutral">Same as the original</Pill>}
    </span>
  );
  const desktopApp = spec.desktop_app ?? "Desktop";
  const desktopWhy = info.desktop_why_not ?? `This account has no ${desktopApp} app profile AI Juggler knows.`;
  // A card that can't be chosen says why on its face (RC-6).
  const why = (words: string) => <span className="mt-1 text-meta text-ink-2">{words}</span>;
  return (
    <Section title="Continue in" testId="switch-where">
      <RadioCards label="Continue in" value={value} onChange={(next) => onChange(next === "desktop" ? "desktop" : "cli")} columns={2}>
        <RadioCard align="start"
          value="desktop"
          logo={<ToolLogo tool={preflight.target_tool} kind="desktop" appId={desktop?.app_id ?? undefined} size={32} />}
          title={title("Desktop", "desktop")}
          sub={desktop ? desktop.name : `${desktopApp} app`}
          disabled={desktop === null}
          reason={desktopWhy}
          testId="switch-where-desktop"
        >
          {desktop === null && why(desktopWhy)}
        </RadioCard>
        <RadioCard align="start"
          value="cli"
          logo={<ToolLogo tool={preflight.target_tool} kind="cli" size={32} />}
          title={title("CLI", "cli")}
          sub={`${spec.display_name} in a terminal`}
          disabled={info.cli_why_not !== null}
          reason={info.cli_why_not ?? undefined}
          testId="switch-where-cli"
        >
          {info.cli_why_not !== null && why(info.cli_why_not)}
        </RadioCard>
      </RadioCards>
      {value === "desktop" && desktop && !desktop.running && (
        <p className="text-meta text-ink-2" data-testid="switch-where-starts">
          AI Juggler starts it as it switches.
        </p>
      )}
    </Section>
  );
}

/** Where it continues, in words: "on Desktop, in the Claude app", "on
 * the CLI, in a terminal". */
export function whereWords(destination: Destination, preflight: PreflightInfoData): string {
  if (destination === "cli") return "in a terminal (CLI)";
  const name = preflight.destinations?.desktop?.name;
  if (name) return `in ${name}`;
  const app = toolSpec(preflight.target_tool).desktop_app;
  return app ? `in the ${app} app (Desktop)` : "in the desktop app";
}

/** The page a switch opens: the Claude app's conversation for a switch
 * that continued in Desktop, else the new session. 0.4.1 (the rc7 walk):
 * a Desktop copy is marked as just opened, so its page reads Starting
 * until the app reports it (`lib/just-opened`). */
export function openedPage(
  result: Pick<SwitchExecutedData, "session_id" | "conversation_key">,
  continuedIn: Destination,
  preflight: Pick<PreflightInfoData, "destinations" | "target_tool">,
): string {
  const key = continuedIn === "desktop" ? result.conversation_key : null;
  const copy = key ? parseSourceKey(key) : null;
  if (!key || !copy) return result.session_id;
  const app = destinationsOf(preflight).desktop?.name ?? toolSpec(preflight.target_tool).desktop_app ?? "its app";
  markOpening(copy.accountId, copy.id, app);
  return key;
}
