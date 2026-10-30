/**
 * AJ-E1504 — every shortcut App owns, matched through the one registry
 * (lib/shortcuts.ts): the `?` sheet is rendered from the same list, and
 * the typing rule lives in `matches`. The map must cover every
 * `owner: "app"` shortcut (the type says so). Handlers read App's latest
 * state through a ref; the listener is set up once.
 */
import { useEffect, useRef } from "react";
import { hasLiveBackend, statsEvent } from "./bridge";
import { matches, type AppShortcutId, type KeyLike } from "./shortcuts";
import { showToast, undoLatest } from "../components/Toast";
import { focusHandover, NO_SESSION_OPEN } from "./handover-focus";

/** AJ-E1103: one keyboard shortcut used (the schema's names). */
export function shortcutUsed(shortcut: string): void {
  statsEvent("ui.shortcut_used", { shortcut });
}

/** What App does for its shortcuts. */
export interface AppKeyActions {
  openShortcuts: () => void;
  togglePalette: () => void;
  /** AJ-E2503: the notification centre (⌥⌘N, D-50). */
  toggleNotifications: () => void;
  /** D-50: ⌘N. */
  newSession: () => void;
  /** D-50: ⌘⇧N. */
  newProject: () => void;
  toggleRail: () => void;
  toggleInspector: () => void;
  cycleDensity: () => void;
  goHome: () => void;
  goTo: (screen: "projects" | "pools" | "tasks" | "accounts" | "usage" | "history" | "fleet" | "cleanup") => void;
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
    notifications: () => {
      shortcutUsed("other");
      d.toggleNotifications();
    },
    // D-50: ⌘N opens New session (deep/11 §3, §4.8); New project is ⌘⇧N
    // (AJ-E305 flow (b)).
    "new-session": () => {
      shortcutUsed("new_session");
      d.newSession();
    },
    "new-project": () => {
      shortcutUsed("other");
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
    // AJ-E2104: the rail owns its places and its View.
    "go-place": (event?: KeyLike) => {
      const digit = event?.code?.startsWith("Digit") ? Number(event.code.slice(5)) : Number(event?.key);
      if (!(digit >= 1 && digit <= 9)) return false;
      shortcutUsed("other");
      window.dispatchEvent(new CustomEvent("aij:go-place", { detail: { index: digit - 1 } }));
    },
    "rail-view": () => {
      shortcutUsed("other");
      window.dispatchEvent(new Event("aij:rail-view"));
    },
    "go-home": () => {
      shortcutUsed("other");
      d.goHome();
    },
    "go-projects": () => {
      shortcutUsed("other");
      d.goTo("projects");
    },
    "go-pools": () => {
      shortcutUsed("other");
      d.goTo("pools");
    },
    "go-tasks": () => {
      shortcutUsed("other");
      d.goTo("tasks");
    },
    "go-accounts": () => {
      shortcutUsed("other");
      d.goTo("accounts");
    },
    "go-usage": () => {
      shortcutUsed("other");
      d.goTo("usage");
    },
    "go-history": () => {
      shortcutUsed("other");
      d.goTo("history");
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
    // AJ-E506/E2404: ⌘⇧C opens Processes; ⌘F focuses its search.
    processes: () => {
      shortcutUsed("other");
      d.goTo("cleanup");
    },
    "processes-search": () => {
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
      // F8: the open session's switch list, any kind; else why not.
      if (!focusHandover()) showToast({ id: "handover", message: NO_SESSION_OPEN });
    },
  } satisfies Record<AppShortcutId, (event?: KeyLike) => boolean | void>;
      // A sheet that is open owns its plain keys (`?` typed in it).
      for (const [id, run] of Object.entries(handlers) as [AppShortcutId, (event?: KeyLike) => boolean | void][]) {
        if (!matches(id, event, hasLiveBackend())) continue;
        if (run(event) === false) return;
        event.preventDefault();
        return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
