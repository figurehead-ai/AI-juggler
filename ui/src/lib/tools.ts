/**
 * AJ-E1525 (ADR-0019 §1): the tools catalog, held once for the whole UI.
 * Screens read a tool's name, sign-in words, install page and first run's
 * groups from its spec, never from a table of their own and never by
 * branching on its id. In the preview (and the tests) the catalog is there
 * from the start; in the app, `loadTools()` runs before the first render.
 */
import { useSyncExternalStore } from "react";
import { hasLiveBackend } from "./bridge";
import { previewTools, toolsCatalog, type ToolSpecInfo } from "./ipc/tools";

let catalog: ToolSpecInfo[] = hasLiveBackend() ? [] : previewTools();
const listeners = new Set<() => void>();

/** Loads the catalog (once at start; again when tools change). */
export async function loadTools(): Promise<void> {
  try {
    catalog = await toolsCatalog();
  } catch {
    // Unreadable: every tool falls back to its id and the letter mark,
    // and nothing is claimed for it.
    catalog = [];
  }
  for (const listener of listeners) listener();
}

/** Test seam: replace the catalog. */
export function setToolsForTests(next: ToolSpecInfo[]): void {
  catalog = next;
  for (const listener of listeners) listener();
}

/** Every registered tool, the primary first. */
export function tools(): ToolSpecInfo[] {
  return catalog;
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};
const snapshot = () => catalog;

/** The catalog, re-rendering when it changes. */
export function useTools(): ToolSpecInfo[] {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}

/** A tool no spec describes: its id as its name, the letter mark, one
 * group, nothing claimed (mirrors `ToolSpec::generic`). */
export function genericTool(id: string): ToolSpecInfo {
  return {
    id,
    display_name: id,
    short_name: id,
    vendor: "",
    mark: null,
    vendor_mark: null,
    install_url: null,
    install_command: null,
    sign_in_own: `${id}'s own sign-in`,
    sign_in_short: `${id}'s`,
    same_as: "as wherever you use it now",
    desktop_app: null,
    kinds: ["cli"],
    rules_files: [],
    scan_hint: "Your default setup and anything else that looks like one. Nothing is changed.",
    setup_groups: [{ id: "setups", title: `${id} setups`, path_contains: null, origin: null, shows_install: true }],
    default_label: null,
    flow: { first_run: false, primary: false },
    terms: null,
    home_env: [],
    supports: { in_place: false, limits: false, app_meter: false, thread_link: false },
  };
}

/** Tool `id`'s spec, or the neutral one. */
export function toolSpec(id: string): ToolSpecInfo {
  return catalog.find((tool) => tool.id === id) ?? genericTool(id);
}

/** The tool a fresh account or a folder import is for when nothing else
 * says; the first tool when none is marked; null with no catalog. */
export function primaryToolId(): string | null {
  return (catalog.find((tool) => tool.flow.primary) ?? catalog[0])?.id ?? null;
}

/** The tools first run on a fresh install brings in, in order. */
export function firstRunToolIds(): string[] {
  return catalog.filter((tool) => tool.flow.first_run).map((tool) => tool.id);
}
