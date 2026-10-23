/**
 * AJ-E1504 — every shortcut App owns, matched through the one registry
 * (lib/shortcuts.ts): the `?` sheet is rendered from the same list, and
 * the typing rule lives in `matches`. The map must cover every
 * `owner: "app"` shortcut (the type says so). Handlers read App's latest
 * state through a ref; the listener is set up once.
 */
import { useEffect, useRef } from "react";
import { hasLiveBackend, statsEvent } from "./bridge";
import { matches, type AppShortcutId } from "./shortcuts";
import { undoLatest } from "../components/Toast";

/** AJ-E1103: one keyboard shortcut used (the schema's names). */
export function shortcutUsed(shortcut: string): void {
  statsEvent("ui.shortcut_used", { shortcut });
}

/** What App does for its shortcuts. */
export interface AppKeyActions {
  openShortcuts: () => void;
  togglePalette: () => void;
  newProject: () => void;
  toggleRail: () => void;
  toggleInspector: () => void;
  cycleDensity: () => void;
  goHome: () => void;
  goTo: (screen: "accounts" | "usage" | "fleet" | "cleanup") => void;
  openSettings: () => void;
  /** Whether the Cleanup Centre is the screen now. */
  onCleanup: () => boolean;
  focusCleanupFilter: () => void;
  stopSelected: () => void;
}

export function useAppKeys(actions: AppKeyActions): void {
  const ref = useRef(actions);
  ref.current = actions;
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const d = ref.current;
      const handlers = {
    help: () => {
      // Never over another sheet (it owns the keys while it's open).
      if (document.querySelector('[aria-modal="true"]')) return false;
      shortcutUsed("other");
      d.openShortcuts();
    },
    // ⌘K toggles the palette (the global binding covers the over-any-app
    // case) and focuses the rail search (AJ-E404 Req.4).
    palette: () => {
      shortcutUsed("command_palette");
      window.dispatchEvent(new Event("aij:focus-search"));
      d.togglePalette();
    },
    // AJ-E305 flow (b).
    "new-project": () => {
      shortcutUsed("new_session");
      d.newProject();
    },
    // The old collapse, kept for muscle memory.
    "toggle-rail": () => {
      shortcutUsed("toggle_rail");
      d.toggleRail();
    },
    inspector: () => {
      shortcutUsed("other");
      d.toggleInspector();
    },
    density: () => {
      shortcutUsed("other");
      d.cycleDensity();
    },
    theme: () => {
      shortcutUsed("other");
      void import("../theme").then((theme) => theme.cycleTheme());
    },
    // AJ-E1503: only when a toast can be undone; otherwise the key passes.
    undo: () => undoLatest(),
    "go-home": () => {
      shortcutUsed("other");
      d.goHome();
    },
    "go-accounts": () => {
      shortcutUsed("other");
      d.goTo("accounts");
    },
    "go-usage": () => {
      shortcutUsed("other");
      d.goTo("usage");
    },
    // AJ-E704: ⌘⇧F (and ⌘4) open Fleet from anywhere.
    "go-fleet": () => {
      shortcutUsed("other");
      d.goTo("fleet");
    },
    // AJ-E1203: ⌘, opens Settings (the macOS convention).
    settings: () => {
      shortcutUsed("other");
      d.openSettings();
    },
    // AJ-E506: ⌘⇧C opens the Cleanup Centre; ⌘F focuses its filter.
    cleanup: () => {
      shortcutUsed("other");
      d.goTo("cleanup");
    },
    "cleanup-filter": () => {
      if (!d.onCleanup()) return false;
      shortcutUsed("search");
      d.focusCleanupFilter();
    },
    // ⌘. stops the selected live Instance (AJ-E407 Actions); ⌘⇧H
    // focuses the hand-over section (RightInspector switches tabs).
    stop: () => {
      shortcutUsed("other");
      d.stopSelected();
    },
    handover: () => {
      shortcutUsed("switch");
      window.dispatchEvent(new Event("aij:focus-handover"));
    },
  } satisfies Record<AppShortcutId, () => boolean | void>;
      // A sheet that is open owns its plain keys (`?` typed in it).
      for (const [id, run] of Object.entries(handlers) as [AppShortcutId, () => boolean | void][]) {
        if (!matches(id, event, hasLiveBackend())) continue;
        if (run() === false) return;
        event.preventDefault();
        return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
