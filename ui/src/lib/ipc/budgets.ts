/**
 * AJ-E1514 — monthly caps. A cap warns in Needs you at 80 % and 100 % of
 * a month's estimated usage; it never stops anything. The month is this
 * screen's local month: the backend keeps no time zone, so every call
 * sends where the month starts here.
 *
 * The preview keeps its caps in `__AIJ_BUDGETS__` (seeded by the gallery
 * and the tests) and records the calls that change them.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { BudgetInfo } from "../../types/generated/BudgetInfo";
import type { BudgetInput } from "../../types/generated/BudgetInput";

export type { BudgetInfo, BudgetInput };

/** Where this local month starts, epoch ms. */
export function monthStartMs(now: Date = new Date()): number {
  return new Date(now.getFullYear(), now.getMonth(), 1).getTime();
}

type Preview = { __AIJ_BUDGETS__?: BudgetInfo[]; __AIJ_BUDGET_CALLS__?: Array<{ call: string; args: unknown }> };
const preview = () => window as unknown as Preview;
const record = (call: string, args: unknown) => (preview().__AIJ_BUDGET_CALLS__ ??= []).push({ call, args });

/** Every cap, with this month's estimate. */
export async function budgetsList(now: Date = new Date()): Promise<BudgetInfo[]> {
  if (!hasLiveBackend()) return preview().__AIJ_BUDGETS__ ?? [];
  return invoke<BudgetInfo[]>("budgets_list", { monthStartMs: monthStartMs(now) });
}

/** Saves a new cap or changes one; its id. */
export async function budgetSave(input: BudgetInput): Promise<string> {
  if (!hasLiveBackend()) {
    record("save", input);
    const id = input.id ?? `cap-preview-${(preview().__AIJ_BUDGETS__ ?? []).length + 1}`;
    const row: BudgetInfo = {
      id,
      name: input.name.trim(),
      monthly_usd: input.monthly_usd,
      account_id: input.account_id,
      project_ids: input.project_ids,
      spent_usd: 0,
      unpriced: false,
      level: "under",
      seen: false,
    };
    const kept = (preview().__AIJ_BUDGETS__ ?? []).filter((cap) => cap.id !== id);
    preview().__AIJ_BUDGETS__ = [...kept, row];
    return id;
  }
  return invoke<string>("budgets_save", { input });
}

/** Removes a cap. */
export async function budgetRemove(id: string): Promise<void> {
  if (!hasLiveBackend()) {
    record("remove", { id });
    preview().__AIJ_BUDGETS__ = (preview().__AIJ_BUDGETS__ ?? []).filter((cap) => cap.id !== id);
    return;
  }
  await invoke<void>("budgets_remove", { id });
}

/** The user has seen a cap's warning at `level` this month. */
export async function budgetSeen(id: string, level: string, now: Date = new Date()): Promise<void> {
  if (!hasLiveBackend()) {
    record("seen", { id, level });
    preview().__AIJ_BUDGETS__ = (preview().__AIJ_BUDGETS__ ?? []).map((cap) => (cap.id === id ? { ...cap, seen: true } : cap));
    return;
  }
  await invoke<void>("budgets_seen", { id, monthStartMs: monthStartMs(now), level });
}

/** "$243" / "$1,200" / "$12.50": whole dollars from $100, cents below. */
export function dollars(usd: number): string {
  return usd >= 100
    ? `$${Math.round(usd).toLocaleString("en-US")}`
    : `$${usd.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

/** The share of the cap used, as a whole percent. */
export function percentOf(cap: BudgetInfo): number {
  return cap.monthly_usd > 0 ? Math.floor(((cap.spent_usd ?? 0) / cap.monthly_usd) * 100) : 0;
}
