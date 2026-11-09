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
