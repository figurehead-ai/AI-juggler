/**
 * AJ-E2108/AJ-E2110 — the "Last messages" switch (Settings › Desktop
 * apps): whether a session's page reads the end of its transcript when
 * it opens. On by default; only "off" is stored. The text is read on
 * this Mac when the page opens, and kept nowhere.
 */
import { appSettingGet, appSettingSet } from "./bridge";

export const SHOW_LAST_MESSAGES_KEY = "show_last_messages";

/** Fired with the new choice when the switch changes. */
export const LAST_MESSAGES_EVENT = "aij:last-messages-changed";

export async function showLastMessages(): Promise<boolean> {
  return (await appSettingGet(SHOW_LAST_MESSAGES_KEY)) !== "0";
}

export async function setShowLastMessages(on: boolean): Promise<void> {
  await appSettingSet(SHOW_LAST_MESSAGES_KEY, on ? "" : "0");
  window.dispatchEvent(new CustomEvent(LAST_MESSAGES_EVENT, { detail: on }));
}
