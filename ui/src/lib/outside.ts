/**
 * What reaches the window from outside it: the system-wide "Go to a
 * session" hotkey (AJ-E414; its stored binding read once) and a clicked
 * notification (AJ-E1510), which opens its session or This Mac.
 */
import { useEffect, useRef } from "react";
import { appSettingGet, onGoToHotkey } from "./ipc/settings";
import { onNotificationClick, type OpenRequest } from "./ipc/notifications";

/** Fired with the new binding ("" = none) when Settings changes it. */
export const GO_TO_HOTKEY_EVENT = "aij:go-to-hotkey";

export function useOutsideRequests(on: { hotkey: (binding: string) => void; goTo: () => void; open: (request: OpenRequest) => void }): void {
  const latest = useRef(on);
  latest.current = on;
  useEffect(() => {
    void appSettingGet("go_to_hotkey").then((stored) => {
      if (stored !== null) latest.current.hotkey(stored);
    });
    // AJ-E2110: Settings › Desktop apps chose (or cleared) the key.
    const changed = (event: Event) => latest.current.hotkey((event as CustomEvent<string>).detail ?? "");
    window.addEventListener(GO_TO_HOTKEY_EVENT, changed);
    const stopGoTo = onGoToHotkey(() => latest.current.goTo());
    const stopClicks = onNotificationClick((request) => latest.current.open(request));
    return () => {
      window.removeEventListener(GO_TO_HOTKEY_EVENT, changed);
      stopGoTo();
      stopClicks();
    };
  }, []);
}

/** Where an outside request leads: the main window's ways in. */
export interface OpenRoutes {
  remote: (machineId: string, sessionId: string | null) => void;
  session: (id: string) => void;
  centre: () => void;
  page: (id: "thismac" | "usage") => void;
  app: (id: string) => void;
  conversation: (accountId: string, id: string) => void;
  pool: (id: string) => void;
  /** AJ-E3519: a Settings pane (the Floor's View menu opens `the-floor`). */
  settings: (section: string) => void;
}

/** Opens what `open` asks for: a notification's session or page, or (AJ-E3510)
 * a door on the Floor: an App, another app's session, a session's subagents
 * (its session), an Account's usage, This Mac. */
export function routeOpen(open: OpenRequest, to: OpenRoutes): void {
  if (open.machine_id) return to.remote(open.machine_id, open.session_id);
  if (open.session_id) return to.session(open.session_id);
  const id = open.id ?? "";
  switch (open.page) {
    case "notifications":
      return to.centre();
    case "app":
      return to.app(id);
    case "conversation":
    case "subagents": {
      if (id.startsWith("s:")) return to.session(id.slice(2));
      const bare = id.startsWith("c:") ? id.slice(2) : id;
      const slash = bare.indexOf("/");
      return slash > 0 ? to.conversation(bare.slice(0, slash), bare.slice(slash + 1)) : to.page("thismac");
    }
    case "usage":
      return to.page("usage");
    case "pool":
      return to.pool(id);
    case "settings":
      return to.settings(id);
    default:
      return to.page("thismac");
  }
}
