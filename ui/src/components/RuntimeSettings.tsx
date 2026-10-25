/**
 * AJ-E2304/E2308 — Settings › Sessions › How sessions run (D-40).
 *
 * - Per launch settings of a tool that can be driven: its tool's default
 *   ("Through Codex's app-server, answer from AI Juggler"), or "Start in
 *   terminal". A Beta runtime is never a standing choice: it is picked per
 *   session, at launch.
 * - "Run without a terminal (Beta)": what lets a session be started that
 *   way at all, off by default, with one line on what changes.
 *
 * Read when Settings › Sessions opens, never at start.
 */
import { useEffect, useState, type JSX } from "react";
import { SettingSwitch as Switch } from "./SettingSwitch";
import { errorText } from "../lib/bridge";
import { runtimeBeta, runtimeChoices, runtimeChoose, type RuntimeChoicesInfo } from "../lib/ipc/runtime";

export const BETA_HINT =
  "Lets you start a Claude Code session with no terminal, its questions and approvals answered here. It runs your own Claude Code with your own sign-in; the terminal stays the default, and Open in terminal hands the conversation back when a turn ends.";

export function RuntimeSettings({ nameOf }: { nameOf?: (profileId: string) => string }): JSX.Element | null {
  const [choices, setChoices] = useState<RuntimeChoicesInfo | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    void runtimeChoices().then(setChoices, () => setChoices(null));
  }, []);
  if (!choices) return null;
  const standing = choices.profiles.filter((p) => !p.beta);
  return (
    <fieldset className="flex flex-col gap-3 border-t border-hairline pt-3" data-testid="settings-runtime">
      <legend className="sr-only">How sessions run</legend>
      <p className="text-xs font-semibold text-ink" aria-hidden="true">
        How sessions run
      </p>
      {standing.map((profile) => (
        <label key={profile.profile_id} className="flex flex-wrap items-center gap-2 text-xs" data-testid={`runtime-${profile.profile_id}`}>
          <span className="min-w-0 flex-1 text-ink">{nameOf?.(profile.profile_id) ?? profile.profile_id}</span>
          <select
            className="h-7 rounded-lg border border-hairline bg-surface-2 px-2 text-xs text-ink"
            value={profile.chosen ?? ""}
            onChange={(event) => {
              const value = event.target.value === "" ? null : (event.target.value as "terminal" | "driven");
              void runtimeChoose(profile.profile_id, value).then(
                (next) => {
                  setChoices(next);
                  setNote(value === "terminal" ? "Its sessions start in a terminal." : "Its sessions run as the tool does by default.");
                },
                (cause) => setNote(errorText(cause)),
              );
            }}
          >
            <option value="">{profile.default === "driven" ? `Through ${profile.label}, answer here` : "In a terminal"} (default)</option>
            {profile.default === "driven" ? <option value="terminal">Start in terminal</option> : <option value="driven">Through {profile.label}, answer here</option>}
          </select>
        </label>
      ))}
      <Switch
        id="runtime_beta"
        label="Run without a terminal (Beta)"
        hint={BETA_HINT}
        checked={choices.beta_on}
        note={note}
        onChange={(on) =>
          void runtimeBeta(on).then(
            (next) => {
              setChoices(next);
              setNote(on ? "You can now pick Run without a terminal when you start a Claude Code session." : "Claude Code sessions run in a terminal.");
            },
            (cause) => setNote(errorText(cause)),
          )
        }
      />
    </fieldset>
  );
}
