/**
 * AJ-E2018: the design preview's data (src/preview/), behind one typed
 * door. The app with a live backend never loads it: `loadPreview` fetches
 * the preview chunk only in a plain browser (the design preview, the
 * Playwright galleries), before the first render, and vitest installs it
 * in its setup. A mock branch reads it with `preview()`, which is only
 * ever called where `hasLiveBackend()` is false.
 */
import { liveBackend } from "./ipc/transport";

/** Everything the mock calls read: one namespace per preview module. */
export type PreviewHooks = typeof import("../preview/hooks");

let hooks: PreviewHooks | null = null;

/** The preview's data. Throws when it isn't loaded: a live backend never
 * reads it, and a plain browser loads it before the first render. */
export function previewHooks(): PreviewHooks {
  if (!hooks) throw new Error("The design preview's data isn't loaded (loadPreview runs before the first render).");
  return hooks;
}

/** Loads the preview's data, only without a live backend. */
export async function loadPreview(): Promise<void> {
  if (liveBackend || hooks) return;
  hooks = await import("../preview/hooks");
}

/** Tests (src/test/setup.ts) install the data directly. */
export function installPreview(data: PreviewHooks): void {
  hooks = data;
}
