/**
 * 0.3.4 — the reads several screens share, kept (lib/shared-read):
 * Home, Accounts, Usage and every session page show the same runways;
 * Home and Usage the same monthly caps; Usage and Fleet the same
 * machines. Each screen still reads again when it opens and on its own
 * timer; a screen opened again shows the last answer in its first frame.
 */
import { quotaDisplayAll, type QuotaDisplayData } from "./bridge";
import { budgetsList, type BudgetInfo } from "./ipc/budgets";
import { fleetUsageByMachine, type MachineUsageInfo } from "./ipc/fleet-launch";
import { sharedRead, useSharedRead } from "./shared-read";

/** Every account's two clocks (`quotaDisplayAll`). Read again when a
 * screen showing them opens, every half minute while one is shown, and
 * by `quotaDisplays.refresh()`. */
export const quotaDisplays = sharedRead<QuotaDisplayData[]>(() => quotaDisplayAll());

const NO_DISPLAYS: QuotaDisplayData[] = [];

/** Every account's runway: `[]` until the first answer (`known` says
 * whether one came). `version` reads again when it changes (Home: the
 * number of accounts). */
export function useQuotaDisplays({ pollMs = 30_000, maxAgeMs = 0, version }: { pollMs?: number; maxAgeMs?: number; version?: unknown } = {}): {
  displays: QuotaDisplayData[];
  known: boolean;
} {
  const displays = useSharedRead(quotaDisplays, { pollMs, maxAgeMs, version });
  return { displays: displays ?? NO_DISPLAYS, known: displays !== undefined };
}

/** Every monthly cap with this month's estimate. Read again when Home or
 * Usage opens, every minute while one is shown, and after a cap is set,
 * removed or seen (`budgets.refresh()`). */
export const budgets = sharedRead<BudgetInfo[]>(() => budgetsList());

const NO_CAPS: BudgetInfo[] = [];

export function useBudgets(): BudgetInfo[] {
  return useBudgetsRead().caps;
}

/** The caps, and whether the first answer has come (`known`). */
export function useBudgetsRead(): { caps: BudgetInfo[]; known: boolean } {
  const caps = useSharedRead(budgets, { pollMs: 60_000 });
  return { caps: caps ?? NO_CAPS, known: caps !== undefined };
}

/** The last day's usage by machine (Usage, Fleet): read again when one
 * opens and every half minute while shown. `null` before the first. */
export const machineUsage = sharedRead<MachineUsageInfo[]>(() => fleetUsageByMachine());

export function useMachineUsage(): MachineUsageInfo[] | null {
  return useSharedRead(machineUsage, { pollMs: 30_000 }) ?? null;
}
