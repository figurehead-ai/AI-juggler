/**
 * AJ-E1611 — whether a tool has a newer version than the one on this Mac
 * (checked once a day, never updated by AI Juggler). The backend's
 * `features::tool_updates`.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { ToolUpdateInfo } from "../../types/generated/ToolUpdateInfo";

export type { ToolUpdateInfo };

/** What `tool`'s page says about its versions; tests and the preview set
 * `window.__AIJ_TOOL_UPDATES__` (by tool id). */
export async function toolUpdateStatus(tool: string): Promise<ToolUpdateInfo | null> {
  if (!hasLiveBackend()) {
    const rows = (window as unknown as { __AIJ_TOOL_UPDATES__?: Record<string, ToolUpdateInfo> }).__AIJ_TOOL_UPDATES__;
    return rows?.[tool] ?? null;
  }
  return invoke<ToolUpdateInfo | null>("tool_updates_status", { tool });
}

/** Turns the once-a-day check on or off. */
export async function toolUpdateCheckSet(on: boolean): Promise<void> {
  if (!hasLiveBackend()) {
    const rows = (window as unknown as { __AIJ_TOOL_UPDATES__?: Record<string, ToolUpdateInfo> }).__AIJ_TOOL_UPDATES__;
    for (const row of Object.values(rows ?? {})) row.checks_on = on;
    return;
  }
  return invoke<void>("tool_updates_set_check", { on });
}
