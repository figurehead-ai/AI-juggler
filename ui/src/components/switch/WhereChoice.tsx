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
import { useMemo, type JSX } from "react";
import type { PreflightInfoData, SwitchExecutedData } from "../../lib/ipc/switch-preflight";
import { parseSourceKey } from "../../lib/ipc/conversations";
import { markOpening } from "../../lib/just-opened";
import type { DestinationsInfo } from "../../types/generated/DestinationsInfo";
import { toolSpec } from "../../lib/tools";
import { AppIcon } from "../AppIcon";
import { IconTerminal } from "../icons";
import { IconComputer } from "../icons-actions";
import { ChoiceOption } from "../controls";

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
  const same = (destination: Destination) =>
    info.suggested === destination && (destination === "cli" || desktop !== null) ? (
      <span className="ml-1.5 rounded-full border border-success/30 bg-success-soft px-1.5 py-px text-label font-medium text-success">Same as the original</span>
    ) : null;
  const desktopApp = spec.desktop_app ?? "Desktop";
  // The app's own icon (its id from the preflight, else the tool's app);
  // a screen when neither is known. Kept across renders, so it's read once.
  const appId = desktop?.app_id ?? spec.host_app ?? null;
  const DesktopIcon = useMemo(
    () =>
      function DesktopIcon(): JSX.Element {
        return appId ? <AppIcon appId={appId} name={desktopApp} size={16} /> : <IconComputer width={15} height={15} />;
      },
    [appId, desktopApp],
  );
  return (
    <div className="flex flex-col gap-2" data-testid="switch-where">
      <span id="switch-where-legend" className="text-meta font-medium text-ink">
        Continue in
      </span>
      <div role="radiogroup" aria-labelledby="switch-where-legend" className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        <ChoiceOption
          type="radio"
          look="card"
          name="switch-where"
          icon={DesktopIcon}
          label={<>Desktop{same("desktop")}</>}
          hint={desktop ? desktop.name : `${desktopApp} app`}
          reason={info.desktop_why_not ?? `This account has no ${desktopApp} app profile AI Juggler knows.`}
          disabled={desktop === null}
          checked={value === "desktop"}
          onChange={() => onChange("desktop")}
          data-testid="switch-where-desktop"
        />
        <ChoiceOption
          type="radio"
          look="card"
          name="switch-where"
          icon={IconTerminal}
          label={<>CLI{same("cli")}</>}
          hint={`${spec.display_name} in a terminal`}
          reason={info.cli_why_not ?? undefined}
          disabled={info.cli_why_not !== null}
          checked={value === "cli"}
          onChange={() => onChange("cli")}
          data-testid="switch-where-cli"
        />
      </div>
      {value === "desktop" && desktop && !desktop.running && (
        <p className="text-label text-muted" data-testid="switch-where-starts">
          AI Juggler starts it as it switches.
        </p>
      )}
    </div>
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
