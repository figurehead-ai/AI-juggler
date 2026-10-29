/**
 * AJ-E2304/E2308 — Settings › Sessions › How sessions run (D-40).
 *
 * - Per launch settings of a tool that can be driven, named as everywhere
 *   ("Work · Default"): its tool's default ("In AI Juggler, answer it
 *   here"), or "In a terminal". A Beta runtime is never a standing choice: it is picked per
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
import { launchTitle } from "../lib/account-words";
import { Picker } from "./controls";

export const BETA_HINT =
  "Lets you start a Claude Code session with no terminal, its questions and approvals answered here. It runs your own Claude Code with your own sign-in; the terminal stays the default, and Open in terminal hands the conversation back when a turn ends.";

/** The two ways a session can run, in words (the 0.2 walk, F12: "Through
 * Codex's app-server"). */
export const RUN_WORDS = { driven: "In AI Juggler, answer it here", terminal: "In a terminal" } as const;

export function RuntimeSettings(): JSX.Element | null {
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
        <div key={profile.profile_id} className="flex flex-wrap items-center gap-2 text-xs" data-testid={`runtime-${profile.profile_id}`}>
          <span className="min-w-0 flex-1 text-ink">{launchTitle({ account_label: profile.account_label, name: profile.name })}</span>
          <Picker
            size="sm"
            ariaLabel={`How ${launchTitle({ account_label: profile.account_label, name: profile.name })} runs`}
            value={profile.chosen ?? ""}
            onChange={(picked) => {
              const value = picked === "" ? null : (picked as "terminal" | "driven");
              void runtimeChoose(profile.profile_id, value).then(
                (next) => {
                  setChoices(next);
                  setNote(value === "terminal" ? "Its sessions start in a terminal." : "Its sessions run as the tool does by default.");
                },
                (cause) => setNote(errorText(cause)),
              );
            }}
            options={[
              { value: "", label: `${RUN_WORDS[profile.default === "driven" ? "driven" : "terminal"]} (default)` },
              profile.default === "driven" ? { value: "terminal", label: RUN_WORDS.terminal } : { value: "driven", label: RUN_WORDS.driven },
            ]}
          />
        </div>
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
