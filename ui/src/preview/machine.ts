/**
 * The preview's This Mac: a tool AI Juggler didn't start, observed (the
 * catalog's primary tool, the only kind This Mac observes today).
 */
import type { ToolTreeInfo } from "../types/generated/ToolTreeInfo";
import { PREVIEW_TOOLS } from "./tools";

export const PREVIEW_OBSERVED: ToolTreeInfo[] = [
  { session_id: null, place: "desktop", tool: PREVIEW_TOOLS[0]!.id, account_label: "Personal", cpu_pct: 14, rss_bytes: 1_600_000_000, helpers: 21, heavy_note: null, memory_estimate: true },
];
