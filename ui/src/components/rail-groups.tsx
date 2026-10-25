/**
 * The rail's groups: folding them (remembered per group), a folded
 * group's "needs you" count, and projects grouped by repository. Moved
 * out of WorkflowRail.tsx (AJ-E1532) so the rail stays within its size.
 */
import type { JSX } from "react";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import { lastWorked } from "../lib/project-words";
import { isSnoozed, unsnoozedNeedsCount, type SnoozeState } from "../lib/railview";
import { attentionBadge } from "./indicator";

const FOLDED_KEY = "aij.rail.folded";

/** An element id from any key (an account label with spaces, a remote
 * path): only safe characters, plus a short hash so two keys can't meet. */
export function safeId(prefix: string, key: string): string {
  let hash = 0;
  for (const char of key) hash = (Math.imul(hash, 31) + char.charCodeAt(0)) | 0;
  return `${prefix}-${key.replace(/[^A-Za-z0-9_-]/g, "-").slice(0, 40)}-${(hash >>> 0).toString(36)}`;
}
export const HISTORY_GROUP = "history-projects";
/** How many history projects show before "Show all". */
export const HISTORY_SHOWN = 5;

export function readFolded(): Set<string> {
  try {
    const raw = JSON.parse(localStorage.getItem(FOLDED_KEY) ?? "[]") as unknown;
    return new Set(Array.isArray(raw) ? raw.filter((id): id is string => typeof id === "string") : []);
  } catch {
    return new Set();
  }
}

export function writeFolded(folded: Set<string>): void {
  try {
    localStorage.setItem(FOLDED_KEY, JSON.stringify([...folded]));
  } catch {
    /* a private window: folding still works for this visit */
  }
}

/** Projects grouped by repository (clones and checkouts of one remote
 * together), the most recently worked-in group first. */
export function repoGroups(projects: ProjectInfo[]): { key: string; name: string; projects: ProjectInfo[] }[] {
  const groups = new Map<string, ProjectInfo[]>();
  for (const project of projects) {
    const key = project.repo ?? `folder:${project.id}`;
    groups.set(key, [...(groups.get(key) ?? []), project]);
  }
  // AJ-E1502: pinned projects lead (a group leads when one of it is).
  const pinned = (project: ProjectInfo) => (project.pinned ? 1 : 0);
  return [...groups.entries()]
    .map(([key, members]) => {
      const sorted = [...members].sort((a, b) => pinned(b) - pinned(a) || lastWorked(b) - lastWorked(a));
      const name = members.length > 1 && key.includes("/") ? (key.split("/").pop() ?? sorted[0]!.name) : sorted[0]!.name;
      return { key, name, projects: sorted };
    })
    .sort((a, b) => pinned(b.projects[0]!) - pinned(a.projects[0]!) || lastWorked(b.projects[0]!) - lastWorked(a.projects[0]!));
}

/**
 * A folded group still says what needs you inside it (0.0.11 walk, Q7):
 * the same "!" mark the session rows' icons carry, and how many. The fold
 * stays the user's choice — nothing unfolds by itself.
 */
export function FoldedNeeds({ rows, snooze, now }: { rows: InstanceInfo[]; snooze: SnoozeState; now: number }): JSX.Element | null {
  const count = unsnoozedNeedsCount(rows, snooze, now);
  if (count === 0) return null;
  const danger = rows.some((row) => !isSnoozed(row, snooze, now) && attentionBadge(row)?.tone === "danger");
  return (
    <span
      className={"flex items-center gap-1 text-[10px] font-semibold " + (danger ? "text-danger" : "text-warning")}
      data-testid="rail-folded-needs"
    >
      <span
        aria-hidden="true"
        className={
          "flex size-3.5 items-center justify-center rounded-full text-[8px] font-bold " +
          (danger ? "bg-danger text-white" : "bg-warning text-black")
        }
      >
        !
      </span>
      <span className="tabular-nums">{count}</span>
      <span className="sr-only">{count === 1 ? "needs you" : "need you"},</span>
    </span>
  );
}

/** A group's fold toggle: a chevron, named for screen readers. */
export function FoldButton({
  open,
  label,
  controls,
  onToggle,
}: {
  open: boolean;
  label: string;
  /** The folded part's id, when it is one element. */
  controls?: string;
  onToggle: () => void;
}): JSX.Element {
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-expanded={open}
      aria-controls={controls}
      aria-label={open ? `Fold ${label}` : `Unfold ${label}`}
      className="flex size-6 shrink-0 items-center justify-center rounded-md text-faint outline-none hover:bg-surface/60 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="rail-fold"
    >
      <svg
        aria-hidden="true"
        width="10"
        height="10"
        viewBox="0 0 10 10"
        className={"transition-transform motion-reduce:transition-none " + (open ? "rotate-90" : "")}
      >
        <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </button>
  );
}

