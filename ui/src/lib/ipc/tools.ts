/**
 * AJ-E1525 — the tools catalog over IPC (ADR-0019 §1): every tool as data.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import { PREVIEW_TOOLS } from "../../preview/tools";
import type { ToolSpecInfo } from "../../types/generated/ToolSpecInfo";

export type { ToolSpecInfo };

/** The preview's catalog (or a test's, through `window.__AIJ_TOOLS__`). */
export function previewTools(): ToolSpecInfo[] {
  return (window as unknown as { __AIJ_TOOLS__?: ToolSpecInfo[] }).__AIJ_TOOLS__ ?? PREVIEW_TOOLS;
}

/** Every registered tool, the primary one first. */
export async function toolsCatalog(): Promise<ToolSpecInfo[]> {
  if (!hasLiveBackend()) return previewTools();
  return invoke<ToolSpecInfo[]>("tools_catalog");
}
