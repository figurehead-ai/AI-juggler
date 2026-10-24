/**
 * What reaches the window from outside it: the system-wide "Go to a
 * session" hotkey (AJ-E414; its stored binding read once) and a clicked
 * notification (AJ-E1510), which opens its session or This Mac.
 */
import { useEffect, useRef } from "react";
import { appSettingGet, onGoToHotkey } from "./bridge";
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
