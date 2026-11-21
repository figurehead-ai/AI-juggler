/**
 * AJ-E706 — the steps of a switch to a paired Mac (CrossMachineSwitchFlow
 * holds the flow and the frame): where it goes and how it moves, the
 * project there, what goes along, and the confirm screen. The same words
 * as this Mac's switch, the tool by its name (D-25). 0.5.0 (D-142): the
 * same rich look as this Mac's: facts on the tinted panel, choices as
 * cards, what blocks as an Alert.
 */
import type { JSX } from "react";
import type { CarryItemData } from "../../lib/ipc/switch-preflight";
import type { RemoteSwitchTargetInfo } from "../../types/generated/RemoteSwitchTargetInfo";
import { toolSpec } from "../../lib/tools";
import { SwitchFreezeChoice, type FreezeChoice } from "../SwitchFreezeChoice";
import { CarryList } from "./CarryList";
import { CheckRow, RadioCard, RadioCards, RowList } from "../controls";
import { Alert } from "../Alert";
import { Pill } from "../Pill";
import { Section } from "../Section";
import { LogoTile } from "../ToolLogo";
import { IconFolderOpen, IconKey, IconPage } from "../icons-actions";
import { IconMessage } from "../icons-canvas";
import { IconCheck } from "../icons-controls";
import { IconWarn } from "../icons-menu";
import { IconShield, IconTerminal } from "../icons";
import { IconHistory } from "../icons-places";
import { Fact, FactPanel } from "./Facts";
import { BadgeTag } from "./TargetCard";

export type Mechanism = "transplant" | "replay";

/** The fidelity badge, under the title, as this Mac's switch shows it. */
export function Badge({ words }: { words: string }): JSX.Element {
  return (
    <p className="flex">
      <BadgeTag badge={words} validated={false} />
    </p>
  );
}

/** A step's heading and its badge. */
function Heading({ title, badge }: { title: string; badge?: string }): JSX.Element {
  return (
    <div className="flex flex-col items-start gap-1.5">
      <h3 className="type-section">{title}</h3>
      {badge && <Badge words={badge} />}
    </div>
  );
}

/** How the work moves, in words: the same words as the badge. */
export const mechanismWords = (mechanism: Mechanism) => (mechanism === "transplant" ? "Same conversation" : "Full story, fresh start");

export function CrossTarget({ target, badge, mechanism, onMechanism, t2Possible }: { target: RemoteSwitchTargetInfo; badge: string; mechanism: Mechanism; onMechanism: (m: Mechanism) => void; t2Possible: boolean }): JSX.Element {
  const machine = target.machine_label;
  const sameVersion = target.version_there !== null && target.version_here === target.version_there;
  const tool = target.provider === "" ? null : toolSpec(target.provider).display_name;
  return (
    <div className="flex flex-col gap-5" data-testid="cross-switch-target">
      <Heading title={`Switch to ${target.account_label} on ${machine}`} badge={badge} />
      <FactPanel testId="cross-switch-preconditions" label="Before it can move">
        {tool && (
          <Fact icon={sameVersion ? IconCheck : IconWarn} tone={sameVersion ? "done" : "warn"} label="Tool">
            {tool} {target.version_here ?? "isn't installed"} here, {target.version_there ?? "not installed"} on {machine}
          </Fact>
        )}
        <Fact icon={target.same_path_project ? IconCheck : IconWarn} tone={target.same_path_project ? "done" : "warn"} label="Project">
          {target.same_path_project ? `The project is at the same path on ${machine}` : `The project isn't at the same path on ${machine}`}
        </Fact>
      </FactPanel>
      {target.blockers.length > 0 && (
        <Alert level="needs-you" live={null} testId="cross-switch-blockers">
          <ul className="flex flex-col gap-0.5">
            {target.blockers.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        </Alert>
      )}
      <Section title="How the work goes along" as="h4">
        <RadioCards label="How the work goes along" value={mechanism} onChange={(next) => onMechanism(next === "replay" ? "replay" : "transplant")} columns={2}>
          <RadioCard
            value="transplant"
            logo={<LogoTile icon={IconMessage} size={32} on={mechanism === "transplant"} />}
            title="Same conversation"
            sub={t2Possible ? `${machine} picks up this conversation where it stopped; its files are copied there.` : "Not possible here: see why above."}
            disabled={!t2Possible}
            reason="Not possible here: see why above."
            testId="cross-switch-t2"
          />
          <RadioCard
            value="replay"
            logo={<LogoTile icon={IconPage} size={32} on={mechanism === "replay"} />}
            title="Full story, fresh start"
            sub={`A new session on ${machine} starts from handover notes of the conversation so far; you choose what happens to keys and passwords.`}
            disabled={!target.replay_available}
            testId="cross-switch-t3"
          />
        </RadioCards>
      </Section>
      <p className="text-meta text-ink-2">Nothing happens until you choose Switch now on the last step.</p>
    </div>
  );
}

export function CrossProject({ target, projectId, onProject, confirmed, onConfirmed }: { target: RemoteSwitchTargetInfo; projectId: string | null; onProject: (id: string) => void; confirmed: boolean; onConfirmed: (on: boolean) => void }): JSX.Element {
  const machine = target.machine_label;
  const projects = [...target.projects].sort((a, b) => Number(b.same_path) - Number(a.same_path) || Number(b.same_name) - Number(a.same_name));
  const mapped = projectId !== null && projectId !== target.same_path_project;
  const name = target.projects.find((project) => project.project_id === projectId)?.name ?? "the project";
  return (
    <div className="flex flex-col gap-5" data-testid="cross-switch-project">
      <div className="flex flex-col gap-1">
        <h3 className="type-section">The project on {machine}</h3>
        <p className="type-help">The project isn't at the same path on {machine}. Pick its copy there; the conversation's paths are changed to it only once you confirm.</p>
      </div>
      {projects.length === 0 ? (
        <Alert level="info" live={null} testId="cross-switch-no-projects">
          {machine} has no projects yet. Open the project there once, then try again.
        </Alert>
      ) : (
        <RadioCards label={`Project on ${machine}`} value={projectId} onChange={onProject}>
          {projects.map((project) => (
            <RadioCard
              key={project.project_id}
              value={project.project_id}
              logo={<LogoTile icon={IconFolderOpen} size={32} on={projectId === project.project_id} />}
              title={
                <span className="inline-flex flex-wrap items-center gap-x-2 gap-y-1">
                  {project.name}
                  {project.same_name && <Pill tone="accent">same name (suggested)</Pill>}
                </span>
              }
              testId="cross-switch-project-option"
            />
          ))}
        </RadioCards>
      )}
      {mapped && (
        <RowList>
          <CheckRow
            icon={IconFolderOpen}
            title={
              <>
                Continue in <span className="font-semibold">{name}</span> on {machine}
              </>
            }
            help="With this conversation's paths changed to it."
            checked={confirmed}
            onChange={onConfirmed}
            testId="cross-switch-confirm-mapping"
          />
        </RowList>
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
    <div className="flex flex-col gap-5" data-testid="cross-switch-carry">
      <Section title="What goes along">
        <CarryList items={carryItems(target)} target={machine} tool={tool} />
      </Section>
      {mechanism === "transplant" ? (
        <p className="text-body text-ink-2">The conversation's files are copied to {machine}; this computer keeps its own copy as it is.</p>
      ) : (
        <Section title="Keys and passwords in this conversation" as="h4" testId="cross-switch-keys">
          <RadioCards label="Keys and passwords in this conversation" value={secrets} onChange={(next) => onSecrets(next === "keep" ? "keep" : "strip")} columns={2}>
            <RadioCard value="strip" logo={<LogoTile icon={IconShield} size={32} on={secrets === "strip"} />} title="Hide them" sub="They're replaced with ••• and never leave this computer. If the other session needs a key, it asks you." testId="cross-switch-secrets-strip" />
            <RadioCard value="keep" logo={<LogoTile icon={IconKey} size={32} on={secrets === "keep"} />} title="Leave them in" sub={`They go to ${machine} exactly as written. Choose this only if the work needs them.`} testId="cross-switch-secrets-keep" />
          </RadioCards>
        </Section>
      )}
    </div>
  );
}

export function CrossConfirm({ target, badge, mechanism, projectName, freeze, onFreeze }: { target: RemoteSwitchTargetInfo; badge: string; mechanism: Mechanism; projectName: string; freeze: FreezeChoice; onFreeze: (choice: FreezeChoice) => void }): JSX.Element {
  const machine = target.machine_label;
  return (
    <div className="flex flex-col gap-5" data-testid="cross-switch-confirm">
      <Heading title={`Ready to switch to ${target.account_label} on ${machine}`} badge={badge} />
      <FactPanel testId="cross-switch-summary" label="What happens">
        <Fact icon={IconTerminal} label="Continues">
          {mechanismWords(mechanism)}: {target.account_label} on {machine} continues it, in {projectName}.
        </Fact>
        <Fact icon={IconMessage} label="Carries">
          {target.dropped.length > 0 ? `Stays behind: ${target.dropped.join(", ")}.` : "Everything goes along."}
        </Fact>
        <Fact icon={IconHistory} label="Original">
          This computer keeps the original as it is.
        </Fact>
      </FactPanel>
      <SwitchFreezeChoice sourceLive value={freeze} onChange={onFreeze} />
    </div>
  );
}
