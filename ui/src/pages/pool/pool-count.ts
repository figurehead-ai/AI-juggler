import { GROUP_PAST } from "./useCanvas";

/** What a pool page counts, said precisely: its members (the sessions AI
 * Juggler runs or carried in), how many of them are open, and, beside
 * them, its project's sessions open in other apps now (the 0.2 walk: "0
 * sessions" while three were open in the Claude app; the rc26 P3 walk:
 * "6 sessions in AI Juggler" with 3 open and 3 ended). */
export function poolCountWords(members: number, open: number, openElsewhere: number): string {
  // Empty, with nothing open elsewhere: its canvas says so, once (design
  // review #85).
  if (members === 0 && openElsewhere === 0) return "";
  const shown = Math.min(open, members);
  const noun = members === 1 ? "session" : "sessions";
  const own =
    members === 0
      ? "0 sessions in AI Juggler"
      : shown === members
        ? `${members} ${noun} open in AI Juggler`
        : shown === 0
          ? `${members} ${noun} in AI Juggler, none open`
          : `${shown} open in AI Juggler · ${members} in all`;
  return openElsewhere > 0 ? `${own} · ${openElsewhere} open in other apps` : own;
}

/** Under a canvas that draws no member while its pool has some (walk K1):
 * their sessions show as they start; with sessions open but none drawn,
 * the diagram grouped them past {@link GROUP_PAST} (F-rc3g-8: never "No
 * one has joined yet" beside the header's "3 open"). */
export function rulesOnlyWords(hasRules: boolean, open: number): string {
  if (open > 0) return `${open} open now, each on its member's card: past ${GROUP_PAST} the diagram groups them. Every one is in the List view.`;
  return hasRules ? "Its sessions show here as they start. Click a member for its role and what it may do." : "None of its sessions is open: they show here as they start, and every one is in the List view.";
}
