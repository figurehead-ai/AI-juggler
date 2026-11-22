/**
 * What clearing and forgetting can't reach (ADR-0037 §1.8), on their
 * own so the gallery's fixtures can read them without the window's code.
 */
/** The clear sheet's backup line (a sibling of `FORGET_AND_BACKUPS`,
 * which speaks of memories): what clearing can't reach. */
export const CLEAR_AND_BACKUPS =
  "Cleared questions are left out of every backup made after this. A backup file you exported before still holds them until you delete that file.";

/** The memory forget sheet's backup words, word for word as
 * `features::ai::memory::FORGET_AND_BACKUPS` (a test holds them equal). */
export const FORGET_AND_BACKUPS =
  "Forgotten memories are left out of every backup made after you forget them. A backup file you exported before still holds them until you delete that file: AI Juggler can't reach a file once it's saved. AI Juggler's own copies, kept before an update or a restore, hold no memories at all.";

