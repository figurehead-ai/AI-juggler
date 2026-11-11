/**
 * 0.4.2 — a link lands on what it names: the view of the screen it opens,
 * by that view's id, carried with the screen's (actions-target-by-id):
 * the top bar's "4 accounts out" opens Accounts filtered to the out ones
 * (`lib/accounts-filter`), "3 to clean up" and "3 processes left running"
 * open Processes on Needs attention. The window's navigation keeps it
 * while that screen is open, so it lands the same whether the screen was
 * open, closed or never drawn; it is never a broadcast to the open page.
 */
import { globalId, type GlobalId } from "./nav";

/** A screen, and the view of it a link named. */
export interface Landing {
  id: GlobalId;
  filter: string;
}

/** The landing an `aij:open-screen` event's detail asks for: its screen
 * and its `filter`; `null` with no filter (the screen as it was left). */
export function landingOf(detail: unknown): Landing | null {
  if (typeof detail !== "object" || detail === null || !("id" in detail) || !("filter" in detail)) return null;
  const id = globalId(detail.id);
  return id && typeof detail.filter === "string" && detail.filter !== "" ? { id, filter: detail.filter } : null;
}

/** Opens screen `id` on its view `filter` (`null`: as it was left). */
export function openScreenOn(id: GlobalId, filter: string | null): void {
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: filter ? { id, filter } : { id } }));
}

/** The view a landing names on screen `id`, else `null`. */
export function landedOn(landing: Landing | null, id: GlobalId): string | null {
  return landing?.id === id ? landing.filter : null;
}

/** Processes' view the "to clean up" links name. */
export const CLEAN_UP_VIEW = "attention";
