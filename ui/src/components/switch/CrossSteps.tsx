/**
 * AJ-E706 — the steps of a switch to a paired Mac (CrossMachineSwitchFlow
 * holds the flow and the frame): where it goes and how it moves, the
 * project there, what goes along, and the confirm screen. The same words
 * as this Mac's switch, the tool by its name (D-25).
 */
import type { JSX } from "react";
import type { CarryItemData, RemoteSwitchTargetInfo } from "../../lib/bridge";
import { toolSpec } from "../../lib/tools";
import { SwitchFreezeChoice, type FreezeChoice } from "../SwitchFreezeChoice";
import { CarryList } from "./CarryList";

export type Mechanism = "transplant" | "replay";

const OPTION = "flex items-start gap-2.5 rounded-lg border px-3 py-2 text-body";
const picked = (on: boolean) => (on ? " border-accent/60 bg-accent-soft" : " border-hairline");

/** The fidelity badge, under the title, as this Mac's switch shows it. */
export function Badge({ words }: { words: string }): JSX.Element {
  return (
    <p className="text-meta text-ink">
      <span className="rounded-md border border-accent/40 px-1.5 py-0.5">{words}</span>
    </p>
  );
}

/** How the work moves, in words: the same words as the badge. */
export const mechanismWords = (mechanism: Mechanism) => (mechanism === "transplant" ? "Same conversation" : "Full story, fresh start");

export function CrossTarget({ target, badge, mechanism, onMechanism, t2Possible }: { target: RemoteSwitchTargetInfo; badge: string; mechanism: Mechanism; onMechanism: (m: Mechanism) => void; t2Possible: boolean }): JSX.Element {
  const machine = target.machine_label;
  const sameVersion = target.version_there !== null && target.version_here === target.version_there;
  const tool = target.provider === "" ? null : toolSpec(target.provider).display_name;
  return (
    <div className="flex flex-col gap-3" data-testid="cross-switch-target">
      <h3 className="text-title text-ink">
        Switch to {target.account_label} on {machine}
      </h3>
      <Badge words={badge} />
      <ul className="flex flex-col gap-1 text-body" data-testid="cross-switch-preconditions">
        {tool && (
          <li className="text-ink">
            <span aria-hidden="true">{sameVersion ? "✓ " : "✗ "}</span>
            {tool} {target.version_here ?? "isn't installed"} here, {target.version_there ?? "not installed"} on {machine}
          </li>
        )}
        <li className="text-ink">
          <span aria-hidden="true">{target.same_path_project ? "✓ " : "✗ "}</span>
          {target.same_path_project ? `The project is at the same path on ${machine}` : `The project isn't at the same path on ${machine}`}
        </li>
      </ul>
      {target.blockers.length > 0 && (
        <ul className="flex flex-col gap-1.5" data-testid="cross-switch-blockers">
          {target.blockers.map((reason) => (
            <li key={reason} className="rounded-lg border border-warning/40 px-3 py-2 text-body text-warning">
              {reason}
            </li>
          ))}
        </ul>
      )}
      <div role="radiogroup" aria-label="How the work moves" className="flex flex-col gap-1.5">
        <label className={OPTION + picked(mechanism === "transplant") + (t2Possible ? "" : " opacity-70")}>
          <input type="radio" name="cross-mechanism" checked={mechanism === "transplant"} disabled={!t2Possible} onChange={() => onMechanism("transplant")} aria-describedby="cross-t2-desc" data-testid="cross-switch-t2" />
          <span>
            <span className="block font-medium text-ink">Same conversation</span>
            <span id="cross-t2-desc" className="text-meta text-ink">
              {t2Possible ? `${machine} picks up this conversation where it stopped; its files are copied there.` : "Not possible here: see why above."}
            </span>
          </span>
        </label>
        <label className={OPTION + picked(mechanism === "replay") + (target.replay_available ? "" : " opacity-70")}>
          <input type="radio" name="cross-mechanism" checked={mechanism === "replay"} disabled={!target.replay_available} onChange={() => onMechanism("replay")} aria-describedby="cross-t3-desc" data-testid="cross-switch-t3" />
          <span>
            <span className="block font-medium text-ink">Full story, fresh start</span>
            <span id="cross-t3-desc" className="text-meta text-ink">
              A new session on {machine} starts from handover notes of the conversation so far; you choose what happens to keys and passwords.
            </span>
          </span>
        </label>
      </div>
      <p className="text-meta text-ink">Nothing moves until you choose Switch now on the last step.</p>
    </div>
  );
}

export function CrossProject({ target, projectId, onProject, confirmed, onConfirmed }: { target: RemoteSwitchTargetInfo; projectId: string | null; onProject: (id: string) => void; confirmed: boolean; onConfirmed: (on: boolean) => void }): JSX.Element {
  const machine = target.machine_label;
  const projects = [...target.projects].sort((a, b) => Number(b.same_path) - Number(a.same_path) || Number(b.same_name) - Number(a.same_name));
  const mapped = projectId !== null && projectId !== target.same_path_project;
  const name = target.projects.find((project) => project.project_id === projectId)?.name ?? "the project";
  return (
    <div className="flex flex-col gap-3" data-testid="cross-switch-project">
      <h3 className="text-title text-ink">The project on {machine}</h3>
      <p className="text-body text-ink">
        The project isn't at the same path on {machine}. Pick its copy there; the conversation's paths are changed to it only once you confirm.
      </p>
      {projects.length === 0 ? (
        <p className="text-body text-ink" data-testid="cross-switch-no-projects">
          {machine} has no projects yet. Open the project there once, then try again.
        </p>
      ) : (
        <div role="radiogroup" aria-label={`Project on ${machine}`} className="flex flex-col gap-1.5">
          {projects.map((project) => (
            <label key={project.project_id} className={OPTION + " items-center" + picked(projectId === project.project_id)}>
              <input type="radio" name="cross-project" checked={projectId === project.project_id} onChange={() => onProject(project.project_id)} data-testid="cross-switch-project-option" />
              <span className="font-medium text-ink">{project.name}</span>
              {project.same_name && <span className="text-meta text-ink">same name (suggested)</span>}
            </label>
          ))}
        </div>
      )}
      {mapped && (
        <label className="flex items-start gap-2.5 text-body text-ink">
          <input type="checkbox" checked={confirmed} onChange={(event) => onConfirmed(event.target.checked)} data-testid="cross-switch-confirm-mapping" />
          <span>
            Continue in <span className="font-medium">{name}</span> on {machine}, with this conversation's paths changed to it.
          </span>
        </label>
      )}
    </div>
  );
}

/** What goes along, in the same list as this Mac's switch. */
function carryItems(target: RemoteSwitchTargetInfo): CarryItemData[] {
  const machine = target.machine_label;
  return [
    ...target.carried.map((label) => ({ label, hint: `It goes to ${machine}.`, how: "moves" as const })),
    ...target.regenerated.map((label) => ({ label, hint: `${machine} makes it again, from its own latest version.`, how: "recreated" as const })),
    ...target.dropped.map((label) => ({ label, hint: "It doesn't go along: the new session starts without it.", how: "stays" as const })),
  ];
}

export function CrossCarried({ target, mechanism, secrets, onSecrets }: { target: RemoteSwitchTargetInfo; mechanism: Mechanism; secrets: "strip" | "keep" | null; onSecrets: (value: "strip" | "keep") => void }): JSX.Element {
  const machine = target.machine_label;
  const tool = target.provider === "" ? "its tool" : toolSpec(target.provider).display_name;
  return (
    <div className="flex flex-col gap-4" data-testid="cross-switch-carry">
      <h3 className="text-title text-ink">What goes along</h3>
      <CarryList items={carryItems(target)} target={machine} tool={tool} />
      {mechanism === "transplant" ? (
        <p className="text-body text-ink">The conversation's files are copied to {machine}; this computer keeps its own copy as it is.</p>
      ) : (
        <fieldset className="flex flex-col gap-1.5" data-testid="cross-switch-keys">
          <legend className="mb-1 text-body font-semibold text-ink">Keys and passwords in this conversation</legend>
          {(
            [
              ["strip", "Hide them", "They're replaced with ••• and never leave this computer. If the other session needs a key, it asks you."],
              ["keep", "Leave them in", `They go to ${machine} exactly as written. Choose this only if the work needs them.`],
            ] as const
          ).map(([value, title, body]) => (
            <label key={value} className={OPTION + picked(secrets === value)}>
              <input type="radio" name="cross-secrets" checked={secrets === value} onChange={() => onSecrets(value)} data-testid={`cross-switch-secrets-${value}`} />
              <span>
                <span className="block font-medium text-ink">{title}</span>
                <span className="text-meta text-ink">{body}</span>
              </span>
            </label>
          ))}
        </fieldset>
      )}
    </div>
  );
}

export function CrossConfirm({ target, badge, mechanism, projectName, freeze, onFreeze }: { target: RemoteSwitchTargetInfo; badge: string; mechanism: Mechanism; projectName: string; freeze: FreezeChoice; onFreeze: (choice: FreezeChoice) => void }): JSX.Element {
  const machine = target.machine_label;
  return (
    <div className="flex flex-col gap-4" data-testid="cross-switch-confirm">
      <h3 className="text-title text-ink">
        Ready to switch to {target.account_label} on {machine}
      </h3>
      <Badge words={badge} />
      <ul className="flex list-disc flex-col gap-1 pl-5 text-body text-ink" data-testid="cross-switch-summary">
        <li>
          {mechanismWords(mechanism)}: {target.account_label} on {machine} continues it, in {projectName}.
        </li>
        <li>{target.dropped.length > 0 ? `Stays behind: ${target.dropped.join(", ")}.` : "Everything goes along."}</li>
        <li>This computer keeps the original as it is.</li>
      </ul>
      <SwitchFreezeChoice sourceLive value={freeze} onChange={onFreeze} />
    </div>
  );
}
