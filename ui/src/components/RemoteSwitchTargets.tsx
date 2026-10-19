/**
 * AJ-E706 — paired machines as switch destinations, in the same handover
 * section as local Accounts. Every row names its machine, carries its
 * fidelity badge in words ("F3 · Unvalidated" — D-04: no machine pair is
 * validated until a live two-machine dogfood passes) and its readiness
 * in words (T2 ready / needs a project mapping / T2 blocked — T3 offered /
 * T3 only / unavailable with the reason). Listing these moved nothing:
 * they come from what each machine published, never a transcript.
 */
import type { JSX } from "react";
import type { RemoteSwitchTargetInfo } from "../lib/bridge";

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
      return "Another Provider — full-story replay";
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
            <p className="text-[11px] font-medium text-ink">On {label}</p>
            <ul className="flex flex-col gap-1">
              {rows.map((target) => {
                const unavailable = target.readiness === "unavailable";
                return (
                  <li key={`${target.machine_id}:${target.recipe_id}`}>
                    <button
                      type="button"
                      disabled={unavailable}
                      onClick={() => onPick(target)}
                      data-testid="remote-switch-target"
                      data-readiness={target.readiness}
                      className={
                        "flex w-full flex-col gap-0.5 rounded-lg border px-2.5 py-1.5 text-left text-[11px] " +
                        (unavailable
                          ? "cursor-not-allowed border-hairline text-muted"
                          : "border-hairline hover:border-accent/40 hover:bg-surface")
                      }
                    >
                      <span className="flex flex-wrap items-center gap-1.5">
                        {/* WCAG 2.5.3: named by its own words. */}
                        <span className="font-medium text-ink">
                          <span className="sr-only">Switch to </span>
                          {target.account_label}
                          {" "}<span className="sr-only">on {label}</span>
                        </span>
                        <span className="text-muted">{target.provider}</span>
                        <span className="text-muted">· {target.recipe_name}</span>
                        <span className="flex-1" />
                        <span
                          className="rounded border border-warning/60 px-1.5 py-px text-[10px] text-warning"
                          data-testid="remote-switch-badge"
                        >
                          {target.badge}
                        </span>
                      </span>
                      <span className={target.readiness === "t2_ready" ? "text-ink" : "text-muted"}>
                        <span aria-hidden="true">
                          {target.readiness === "t2_ready" ? "✓ " : unavailable ? "○ " : "◐ "}
                        </span>
                        {readinessText(target)}
                      </span>
                    </button>
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
