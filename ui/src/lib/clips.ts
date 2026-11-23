/**
 * AJ-E1546 — the short clips that show what a card's words promise: every
 * "What it does for you" card, every "How it works" step and each
 * permission card. Each is a 3–6 s loop recorded from the gallery's
 * generic fixtures (`ui/tests/clips/`), muted, in both themes, with a
 * still poster for Reduce Motion; the files live in `public/clips/` and
 * load only when their card is on screen.
 *
 * The ids are the files' names: `<id>-<theme>.webm` and `<id>-<theme>.png`.
 */

export const CLIP_IDS = [
  "what-bot",
  "what-sessions",
  "what-usage",
  "what-mac",
  "what-setups",
  "how-apps",
  "how-start",
  "how-juggle",
  "perm-focus",
  "perm-preview",
] as const;

export type ClipId = (typeof CLIP_IDS)[number];

export type ClipTheme = "dark" | "light";

/** Whether `id` names a recorded clip. */
export function isClip(id: string): id is ClipId {
  return (CLIP_IDS as readonly string[]).includes(id);
}

/** The clip showing what each system prompt's permission does (AJ-E1547's
 * registry ids): bringing a window forward, or a live preview. */
const PROMPT_CLIPS: Record<string, ClipId> = {
  accessibility: "perm-focus",
  "automation-system-events": "perm-focus",
  "automation-terminal": "perm-focus",
  "automation-iterm": "perm-focus",
  "screen-recording": "perm-preview",
};

/** A permission card's clip, when one shows what it does. */
export function permissionClip(promptId: string): ClipId | null {
  return PROMPT_CLIPS[promptId] ?? null;
}

/** The clip's files for a theme. */
export function clipFiles(id: ClipId, theme: ClipTheme): { video: string; poster: string } {
  const base = `${import.meta.env.BASE_URL ?? "/"}clips/${id}-${theme}`;
  return { video: `${base}.webm`, poster: `${base}.png` };
}

/** How many times a clip plays through before it rests on its last frame
 * (MOT-2: nothing moves for ever; Play starts it again). */
export const CLIP_PLAYS = 3;
