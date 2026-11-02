/**
 * AJ-E1514 — a monthly cap in Needs you: at 80 % ("near") and 100 %
 * ("over") of its amount, until the user says they've seen it (then
 * again at the next level, or next month). It says so and offers Usage;
 * nothing is stopped, and nothing here could stop anything.
 */
import { useMemo, type JSX } from "react";
import { Button } from "./primitives";
import { scopeWords, spentWords } from "./BudgetCaps";
import type { AccountInfo, ProjectInfo } from "../lib/bridge";
import { budgetSeen, percentOf, type BudgetInfo } from "../lib/ipc/budgets";
import { budgets, useBudgetsRead } from "../lib/kept-reads";

/** The caps Needs you shows now (refreshed every minute; the last
 * answer, Usage's too, shows at once). */
export function useCapNeeds(): { caps: BudgetInfo[]; known: boolean; reload: () => void } {
  const { caps: all, known } = useBudgetsRead();
  const caps = useMemo(() => all.filter((cap) => cap.level !== "under" && !cap.seen), [all]);
  return { caps, known, reload: reloadBudgets };
}

const reloadBudgets = () => void budgets.refresh();

/** A cap's warning, in words. */
export function capTitle(cap: BudgetInfo): string {
  return cap.level === "over" ? `${cap.name} is over its monthly cap` : `${cap.name} is at ${percentOf(cap)}% of its monthly cap`;
}

/** Home's headline when a cap is what needs you. */
export function capHeadline(caps: BudgetInfo[]): string {
  return caps.length === 1 ? capTitle(caps[0]!) : `${caps.length} monthly caps need a look`;
}

export function CapNeedItems({
  caps,
  accounts,
  projects,
  onSeen,
}: {
  caps: BudgetInfo[];
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  onSeen: () => void;
}): JSX.Element | null {
  if (caps.length === 0) return null;
  // Its own list: Home places these after Needs you's list (axe: an
  // <li> must sit in a list).
  return (
    <ul className="flex flex-col gap-2.5" aria-label="Spending caps">
      {caps.map((cap) => {
        const over = cap.level === "over";
        return (
          <li key={`cap-${cap.id}`} data-home-item className="flex flex-wrap items-center gap-x-3.5 gap-y-2 rounded-xl border border-hairline bg-surface px-4 py-3.5" data-testid="home-cap">
            <span aria-hidden="true" className={`flex size-8 shrink-0 items-center justify-center rounded-lg font-bold ${over ? "bg-danger-soft text-danger" : "bg-warning-soft text-warning"}`}>
              {over ? "■" : "▲"}
            </span>
            <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
              <span className="text-section font-medium text-ink">{capTitle(cap)}</span>
              <span className="text-xs text-muted">
                {spentWords(cap)} · {scopeWords(cap, accounts, projects)}. Nothing is stopped.
              </span>
            </span>
            <span className="ml-auto flex gap-2">
              <Button size="sm" variant="ghost" onClick={() => void budgetSeen(cap.id, cap.level).then(onSeen, () => null)}>
                Got it <span className="sr-only">— {cap.name}</span>
              </Button>
              <Button size="sm" variant="secondary" onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "usage" } }))}>
                See usage
              </Button>
            </span>
          </li>
        );
      })}
    </ul>
  );
}
