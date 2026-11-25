/**
 * rc21: a conversation its tool won't resume by its id (a copy an older
 * AI Juggler made) carries its tool's words for why, from the backend's
 * one check (`features::launch::resume_gate`). Every door that resumes it
 * (Resume, Continue in AI Juggler, Copy the resume command, a menu's or
 * ⌘K's) says those words in place, and none acts.
 */
import type { ResumeRefusalInfo } from "../types/generated/ResumeRefusalInfo";

export type { ResumeRefusalInfo };

/** Why it can't be resumed, with how to find the original when AI
 * Juggler doesn't know it; `null` when it can. */
export function resumeRefusalWords(refusal: ResumeRefusalInfo | null | undefined): string | null {
  if (!refusal) return null;
  return refusal.find ? `${refusal.why}. ${refusal.find}` : refusal.why;
}

/** The backend's own words when a conversation is open in another
 * session (the Claude app, a terminal): Continue in AI Juggler refuses it. */
export const OPEN_ELSEWHERE = "It's open in another session now: close it there first, then continue here";

/** Why "Continue in AI Juggler" can't run now, said at the button (the
 * rc31 walk: with the conversation open in the Claude app the reason was
 * only in app.log): open in another session, or its tool's refusal;
 * `null` when it can. */
export function continueWhy(conversation: { live: boolean; resume_refusal?: ResumeRefusalInfo | null }): string | null {
  return conversation.live ? OPEN_ELSEWHERE : resumeRefusalWords(conversation.resume_refusal);
}

/** AJ-E2326 R3: a move holds the conversation (it runs, or it was refused
 * or undone a moment ago and the conversation is opening again where it
 * was): Continue in AI Juggler isn't offered, the move's next steps are. */
export function heldByMove(conversation: { resume_refusal?: ResumeRefusalInfo | null }): boolean {
  return conversation.resume_refusal?.moving === true;
}

/** AJ-E2326 R3: what Continue in AI Juggler does, said on it when the
 * conversation's own app is the page's primary. */
export const CONTINUE_IN_TERMINAL = "Starts it in a terminal here, on this account";
