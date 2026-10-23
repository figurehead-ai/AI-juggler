/**
 * What reaches the window from outside it: the system-wide "Go to a
 * session" hotkey (AJ-E414; its stored binding read once) and a clicked
 * notification (AJ-E1510), which opens its session or This Mac.
 */
import { useEffect, useRef } from "react";
import { appSettingGet, onGoToHotkey } from "./bridge";
import { onNotificationClick, type OpenRequest } from "./ipc/notifications";

export function useOutsideRequests(on: { hotkey: (binding: string) => void; goTo: () => void; open: (request: OpenRequest) => void }): void {
  const latest = useRef(on);
  latest.current = on;
  useEffect(() => {
    void appSettingGet("go_to_hotkey").then((stored) => {
      if (stored !== null) latest.current.hotkey(stored);
    });
    const stopGoTo = onGoToHotkey(() => latest.current.goTo());
    const stopClicks = onNotificationClick((request) => latest.current.open(request));
    return () => {
      stopGoTo();
      stopClicks();
    };
  }, []);
}
