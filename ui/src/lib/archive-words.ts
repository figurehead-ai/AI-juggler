/**
 * 0.4.1: the words for another app's session archived in AI Juggler (its
 * toast and its Archived mark): hidden here, still listed in its app.
 */
import type { ConversationInfo } from "./ipc/conversations";
import { conversationItem } from "./rail-items";

/** 0.4.1: what an archive in AI Juggler of another app's session means,
 * as its Archived mark says it: "Archived in AI Juggler; still listed in
 * Claude". `app`: the desktop app it was last open in, when known (a
 * closed one's row names only its tool; the rc8 walk read "Claude Code"). */
export function archivedHereWords(c: ConversationInfo, app?: string | null): string {
  return `Archived in AI Juggler; ${stillListedWords(c, app)}`;
}

/** "still listed in Claude": the app that still lists it. */
export function stillListedWords(c: ConversationInfo, app?: string | null): string {
  return `still listed in ${app || conversationItem(c, () => null, null).place.app}`;
}
