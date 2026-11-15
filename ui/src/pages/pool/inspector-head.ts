/**
 * 0.4.6 (AJ-E3012 §9): the inspector always leads with what is selected —
 * its own title, its kind and App as a subtitle ("Jive CLI" / "Session ·
 * Claude Code · Member"), and its status as a pill under the title, never
 * the heading. That covers a session, an App, an Account, a Tool, a Mac,
 * everyone in scope, a Watched session, a line between two members, the
 * lead's placeholder, and the pool itself when nothing is selected.
 */
import type { AttentionLevel } from "../../lib/attention";
import { MACHINE_WORD } from "../../lib/machine-words";
import { roleCard } from "../../lib/pool-words";

export interface InspectorHead {
  title: string;
  subtitle: string;
  /** Its status, with its attention level (L0 calm … L3 broken). */
  pill: { words: string; level: AttentionLevel } | null;
}

/** A member kind's word in the subtitle. */
const KIND_WORDS: Record<string, string> = { session: "Session", app: "App", account: "Account", tool: "Tool", mac: MACHINE_WORD, everyone: "Everyone in scope" };

export type HeadOf =
  | { kind: "pool"; name: string; own: string | null; where: string; sessions: number; sealed: boolean; needs: number }
  | { kind: "session"; name: string; app: string; role: string; state: string; level: AttentionLevel }
  | { kind: "rule"; label: string; memberKind: string; role: string; open: number; leadsNext?: boolean }
  | { kind: "watched"; title: string; app: string | null; role: string }
  | { kind: "edge"; from: string; to: string; count: string; waits: boolean }
  | { kind: "lead-next"; app: string };

export function inspectorHead(of: HeadOf): InspectorHead {
  switch (of.kind) {
    case "pool":
      return {
        title: of.name,
        subtitle: `Pool · ${of.own ? `${of.own}'s own pool: every session in ${of.own}, as one team` : `in ${of.where}`}`,
        pill: of.needs > 0 ? { words: `${of.needs} need${of.needs === 1 ? "s" : ""} you`, level: 2 } : { words: of.sealed ? "Sealed" : `${of.sessions} ${of.sessions === 1 ? "session" : "sessions"}`, level: 0 },
      };
    case "session":
      return { title: of.name, subtitle: `Session · ${of.app} · ${roleCard(of.role).label}`, pill: { words: of.state, level: of.level } };
    case "rule":
      return {
        title: of.label,
        // Round 2 (#2): an App whose next session leads says so, not "Member".
        subtitle: `${KIND_WORDS[of.memberKind] ?? "Member"} · ${of.leadsNext ? "Leads next" : roleCard(of.role).label}`,
        pill: { words: of.open > 0 ? `${of.open} open now` : "None open now", level: 0 },
      };
    case "watched":
      return { title: of.title, subtitle: `Session · ${of.app ?? "another app"} · Watched · ${roleCard(of.role).label}`, pill: { words: "Can't reply here yet", level: 1 } };
    case "edge":
      return { title: `${of.from} → ${of.to}`, subtitle: "Line · what passes between these two", pill: of.waits ? { words: "Waits on you", level: 2 } : { words: of.count, level: 0 } };
    default:
      return { title: `${of.app}'s next session`, subtitle: "Lead · it leads once it starts here", pill: { words: "Not started yet", level: 0 } };
  }
}
