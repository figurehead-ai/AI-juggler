/**
 * D-112: the main window's close button and ⌘W hide it, and AI Juggler
 * keeps running. The first time, it says so once, as a toast, the next
 * time the window is on screen (a hidden window can't show one). The
 * backend says whether it is due (a close hid the window in this run and
 * it was never said) and records it once said, so it never shows again
 * on this install.
 */
import { showToast } from "../components/Toast";
import { appCloseNoteDue, appCloseNoteTold } from "./ipc/app-lifecycle";
import { whenShownAgain } from "./shown-window";

/** Says it if it is due, then tells the backend it was said. */
async function sayIfDue(): Promise<void> {
  if (!(await appCloseNoteDue())) return;
  showToast({
    id: "close-note",
    message: "AI Juggler kept running while its window was closed.",
    detail: "It stays in the menu bar. ⌘Q quits.",
    seconds: 10,
    testId: "close-note",
  });
  await appCloseNoteTold();
}

/** Watches for the window coming back on screen; returns the stop. */
export function watchCloseNote(): () => void {
  let asking = false;
  return whenShownAgain(() => {
    if (asking) return;
    asking = true;
    void sayIfDue()
      .catch(() => undefined)
      .finally(() => {
        asking = false;
      });
  });
}
