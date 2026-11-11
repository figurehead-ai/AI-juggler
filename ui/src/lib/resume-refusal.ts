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
