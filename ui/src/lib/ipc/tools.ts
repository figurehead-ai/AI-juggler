/**
 * AJ-E1525 — the tools catalog over IPC (ADR-0019 §1): every tool as data.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { ToolSpecInfo } from "../../types/generated/ToolSpecInfo";
import { previewHooks } from "../preview-hooks";

export type { ToolSpecInfo };

/** The preview's catalog (or a test's, through `window.__AIJ_TOOLS__`). */
export function previewTools(): ToolSpecInfo[] {
  return (window as unknown as { __AIJ_TOOLS__?: ToolSpecInfo[] }).__AIJ_TOOLS__ ?? previewHooks().tools.PREVIEW_TOOLS;
}

/** Every registered tool, the primary one first. */
export async function toolsCatalog(): Promise<ToolSpecInfo[]> {
  if (!hasLiveBackend()) return previewTools();
  return call("tools_catalog");
}
