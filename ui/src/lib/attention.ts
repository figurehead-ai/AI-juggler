/**
 * The words for an attention rule (D-25: never the rule's code), wherever
 * a row or a header says why a session needs you or ended as it did.
 */
export function attentionWords(rule: string | null | undefined): string {
  switch (rule) {
    case "waiting_prompt":
    case "waiting_user":
      return "waiting for your reply";
    // AJ-E2306/E2303: the tool itself said it asks (its hooks or protocol).
    case "tool_asks":
      return "asks you";
    case "stuck_no_output":
    case "stuck":
      return "no output for a while";
    case "quota_exit":
    case "quota_blocked":
    case "quota_wall":
    // 0.3: open at its limit (the tool's own record; it waits there).
    case "limit_hit":
      return "hit its usage limit";
    case "failed":
      return "stopped with an error";
    // AJ-E1509: why a crashed session wasn't started again, and a session
    // that ended with AI Juggler.
    case "restart_gave_up":
      return "crashed 3 times in 10 minutes, so it wasn't restarted";
    case "app_quit_unexpectedly":
      return "ended when AI Juggler quit unexpectedly";
    // AJ-E2310: a driven session AI Juggler stopped during its turn.
    case "turn_cut_off":
      return "its turn was cut off when AI Juggler stopped";
    // A session whose process never started (the reason is its input).
    case "launch_failed":
      return "couldn't start";
    default:
      return "needs attention";
  }
}

/**
 * 0.5 (D-117, the attention budget; amends D-47 point 2) — a thing's
 * attention level, the ONE rule every view reads: the rail, Home's marks,
 * the bell, the menu-bar item and the app badge. The backend counts with
 * its mirror (`features/notifications/centre.rs` `attention_level`); both
 * are held to the same table (`fixtures/attention-levels.json`).
 *
 * ISA-18.2's consequence × time to respond, for supervised AI work:
 * - **L3 Broken**: work stopped or will be lost, and only you can recover
 *   it (stopped with an error; a limit with no Account to go on; an
 *   Account out while a session runs on it; this Mac at critical pressure).
 * - **L2 Needs you**: work waits on a decision; nothing is lost while it
 *   waits (asks, probably asks, probably stuck, a limit with room
 *   elsewhere, cut off, a sign-in, a cap, an agent's message or task, a
 *   session working the Mac hard, the Mac under pressure, an Account at
 *   10 % or less under a live session).
 * - **L1 Note**: good to know, no action now (done, review it; low room
 *   nothing live depends on; processes left running).
 * - **L0 Calm**: the running picture (working, idle, ended), and a
 *   session parked until its reset, whatever its need: AI Juggler holds
 *   it, nothing is lost, and the reset raises its one offer (a notice,
 *   AJ-E1003). A need at L0 is no need: the backend lists none for it.
 *
 * Inferred states stay at their level (the word says "Probably"); they
 * draw a hollow ring instead of a filled dot (`level-mark`).
 */
export type AttentionLevel = 0 | 1 | 2 | 3;

/** What a level is worked out from. `need` is a Needs-you kind, the
 * backend's notice kinds and the rail's needs alike (`asks`, `stuck`,
 * `limit`, `failed`, `review`, `cut_off`, `signin`, `room`, `heavy`,
 * `leftovers`, `cap`, `message`, `task`, `quota`); with none, `state`. */
export interface AttentionSubject {
  need?: string | null;
  /** A session's state in the rail's words (`working`, `done_review`…). */
  state?: string | null;
  /** quota: the room left, in percent. */
  left?: number | null;
  /** quota: live sessions on the Account. */
  liveSessions?: number | null;
  /** limit: no Account has room to go on. */
  noRoom?: boolean;
  /** room: this Mac at critical pressure. */
  critical?: boolean;
  /** A session parked until its reset (AJ-E1003): calm (L0), whatever
   * its need. The CI run 43777785232 home-scrolled capture: its parked
   * session's "Limit reached" lit the folded rail while the count, the
   * backend's (which lists no parked session), left it out. */
  parked?: boolean;
}

/** The kinds that are a decision waiting (L2) whatever their facts. */
const WAITS = new Set(["asks", "stuck", "cut_off", "signin", "cap", "message", "task", "heavy"]);
/** The kinds that are a note (L1). */
const NOTES = new Set(["review", "leftovers"]);

/** Its level (see above). An unknown need is a need (L2): a new kind is
 * never quieter than it should be until it is placed here. */
export function attentionLevel(subject: AttentionSubject): AttentionLevel {
  const { need } = subject;
  if (subject.parked) return 0;
  if (need) {
    if (need === "failed") return 3;
    if (need === "limit") return subject.noRoom ? 3 : 2;
    if (need === "room") return subject.critical ? 3 : 2;
    if (need === "quota") {
      const live = (subject.liveSessions ?? 0) > 0;
      const left = subject.left ?? null;
      if (live && left !== null && left <= 0) return 3;
      if (live && left !== null && left <= 10) return 2;
      return 1;
    }
    if (NOTES.has(need)) return 1;
    if (WAITS.has(need)) return 2;
    return 2;
  }
  return subject.state === "done_review" ? 1 : subject.state === "asks_you" ? 2 : 0;
}

/** A rail item's level: its need, else its state. */
export function itemLevel(item: { need: string | null; state: string }): AttentionLevel {
  return attentionLevel({ need: item.need, state: item.state });
}

/** The shared count (the rail's Needs you, the bell, Home's title, the
 * badge and the menu-bar item): L2 and L3 only. "To review" (L1) stays
 * listed, last, and uncounted. */
export function counted(level: AttentionLevel): boolean {
  return level >= 2;
}

/** A notice's level (the backend's Needs-you row: its kind, meter and
 * the live sessions its facts name). */
export function noticeLevel(notice: { kind: string; meter?: number | null; facts?: { sessions?: number | null } | null }): AttentionLevel {
  return attentionLevel({ need: notice.kind, left: notice.meter ?? null, liveSessions: notice.facts?.sessions ?? null });
}

/** A level's colour as text and as a dot (the classes, never a hex):
 * amber at L2, red at L3, else neutral (L1's hue is on its glyph only). */
export function levelTone(level: AttentionLevel): { text: string; dot: string } {
  if (level === 3) return { text: "text-danger", dot: "bg-danger" };
  if (level === 2) return { text: "text-warning", dot: "bg-warning" };
  return { text: "text-muted", dot: "bg-faint" };
}

/** The highest level among `levels` (a count takes the colour of the
 * highest level it holds), 0 for none. */
export function highest(levels: Iterable<AttentionLevel>): AttentionLevel {
  let top: AttentionLevel = 0;
  for (const level of levels) if (level > top) top = level;
  return top;
}

/** An App's icon or mark tone (D-117, `.app-tone` in styles.css): muted
 * at rest, in full colour when `lit` (L2/L3, selected or current); a
 * `tone-host` around it lights it on hover and focus too. */
export function appTone(lit: boolean): string {
  return lit ? "app-tone is-lit" : "app-tone";
}

/** D-117, said aloud (the a11y fix of 2026-11-15, Happo's snapshots of
 * 83f700ed6: the head read "Needs you ✕ 5"): a level's glyphs (⚠ ✕, the
 * dot, the ring) are decoration, hidden from screen readers; the words
 * carry the level. What an L3 (broken) thing is, in its row's words. */
export function brokenWord(need: string | null | undefined): string {
  switch (need) {
    case "failed":
      return "stopped with an error";
    case "limit":
      return "limit reached";
    case "quota":
      return "out of room";
    case "room":
      return "this computer under critical pressure";
    default:
      return "broken";
  }
}

/** Each broken (L3) thing's word among `things` (its level by `levelOf`,
 * its kind by `needOf`). */
export function brokenOf<T>(things: readonly T[], levelOf: (thing: T) => AttentionLevel, needOf: (thing: T) => string | null | undefined): string[] {
  return things.filter((thing) => levelOf(thing) === 3).map((thing) => brokenWord(needOf(thing)));
}

/** The broken (L3) things by their words, most first: "2 stopped with an
 * error, 1 out of room"; empty for none. */
export function brokenParts(words: readonly string[]): string {
  const counts = new Map<string, number>();
  for (const word of words) counts.set(word, (counts.get(word) ?? 0) + 1);
  return [...counts]
    .sort((a, b) => b[1] - a[1])
    .map(([word, n]) => `${n} ${word}`)
    .join(", ");
}

/** A level count, said aloud: the number, then what broke (L3), then what
 * waits on you (L2) when both are there: "5, 2 stopped with an error, 3
 * wait on you"; "3" when nothing broke. `broken`: each L3 thing's word. */
export function levelCountWords(count: number, broken: readonly string[]): string {
  if (broken.length === 0) return String(count);
  const waits = count - broken.length;
  const rest = waits > 0 ? `, ${waits} ${waits === 1 ? "waits" : "wait"} on you` : "";
  return `${count}, ${brokenParts(broken)}${rest}`;
}
