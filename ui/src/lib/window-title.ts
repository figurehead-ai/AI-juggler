/**
 * AJ-E2803 R4: the window says where you are, "AI Juggler · Projects", as
 * the header's lit tab does: the same screen name, in the window's title
 * (the Dock, ⌘`, Mission Control) and the page's.
 */
import { useEffect } from "react";
import { liveBackend } from "./ipc/transport";

/** The window's title for `screen` (none: the app's name alone). */
export function windowTitle(screen: string): string {
  const name = screen.trim();
  return name ? `AI Juggler · ${name}` : "AI Juggler";
}

/** Keeps the window's title on `screen`. */
export function useWindowTitle(screen: string): void {
  useEffect(() => {
    const title = windowTitle(screen);
    document.title = title;
    if (!liveBackend) return;
    void import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => getCurrentWindow().setTitle(title))
      .catch(() => {
        // The page's title stays right; the window's keeps its last.
      });
  }, [screen]);
}
// CI probe (AJ-E2020 R2): a UI-only change; never merged.
