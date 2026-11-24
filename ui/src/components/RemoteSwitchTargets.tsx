/**
 * AJ-E706 — paired machines as switch destinations, in the same handover
 * section as local Accounts. Every row names its machine, carries its
 * fidelity badge in words ("Same conversation, another app · Beta" — D-04: no machine pair is
 * validated until a live two-machine dogfood passes, so D-127 lists them only with beta switches on
 * in Settings › Switching) and its readiness
 * in words (T2 ready / needs a project mapping / T2 blocked — T3 offered /
 * T3 only / unavailable with the reason). Listing these moved nothing:
 * they come from what each machine published, never a transcript.
 */
import { cliAppName } from "../lib/app-name";
import type { JSX } from "react";
import type { RemoteSwitchTargetInfo } from "../types/generated/RemoteSwitchTargetInfo";
import { launchSettingsName } from "../lib/account-words";
import { BadgeLine, TargetButton, TargetLogo } from "./switch/TargetCard";

/** Readiness in plain words (the row's second line). */
export function readinessText(target: RemoteSwitchTargetInfo): string {
  switch (target.readiness) {
    case "t2_ready":
      return "Ready — same version, same project path";
    case "needs_mapping":
      return "Needs you to pick the project there";
    case "t2_blocked":
      return "Same conversation can't move — full-story replay offered";
    case "t3_only":
      return "Another tool — full-story replay";
    default:
      return target.blockers[0] ?? "Unavailable";
  }
}

export function RemoteSwitchTargets({
  targets,
  onPick,
}: {
  targets: RemoteSwitchTargetInfo[];
  onPick: (target: RemoteSwitchTargetInfo) => void;
}): JSX.Element {
  const machines = Array.from(new Set(targets.map((target) => target.machine_id)));
  return (
    <div className="flex flex-col gap-2" data-testid="remote-switch-targets">
      {machines.map((machineId) => {
        const rows = targets.filter((target) => target.machine_id === machineId);
        const label = rows[0]?.machine_label ?? machineId;
        return (
          <section key={machineId} aria-label={`Destinations on ${label}`} className="flex flex-col gap-1">
            <p className="text-label font-medium text-ink">On {label}</p>
            <ul className="flex flex-col gap-2">
              {rows.map((target) => {
                const unavailable = target.readiness === "unavailable";
                return (
                  <li key={`${target.machine_id}:${target.recipe_id}`}>
                    {/* 0.5.1 B2 critique: the same card as a target on this
                        Mac (TargetCard: logo, the App's name, a tone step,
                        no line), how much carries a quiet line, never an
                        amber-ringed chip on every row. */}
                    <TargetButton
                      disabled={unavailable}
                      onClick={() => onPick(target)}
                      data-testid="remote-switch-target"
                      data-readiness={target.readiness}
                      logo={<TargetLogo tool={target.provider} kind="cli" folderId={`${target.machine_id}:${target.recipe_id}`} label={target.account_label} size={32} />}
                      // The App whole, as the rows on this Mac name it (the 0.4 review).
                      name={cliAppName(target.provider, target.account_label)}
                      sub={
                        <>
                          {/* WCAG 2.5.3: named by its own words; the machine for a screen reader. */}
                          <span className="sr-only">on {label} · </span>
                          {launchSettingsName(target.recipe_name)}
                        </>
                      }
                      detail={
                        <>
                          <span data-testid="remote-switch-badge" className="contents">
                            <BadgeLine badge={target.badge} validated={false} />
                          </span>
                          <span className={target.readiness === "t2_ready" ? "text-ink" : "text-ink-2"}>
                            <span aria-hidden="true">{target.readiness === "t2_ready" ? "✓ " : unavailable ? "○ " : "◐ "}</span>
                            {readinessText(target)}
                          </span>
                        </>
                      }
                    />
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
    </div>
  );
}
