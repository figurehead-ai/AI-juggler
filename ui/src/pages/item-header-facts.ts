/**
 * A tool's and a session's facts as the item header says them: how many
 * of a tool's sessions run and work (AJ-E1529), and a session state as a
 * chip. Pure, so the header, the inspector and the tool page share them.
 */
import { STATE_WORDS, isActiveStatus, stateChip } from "../components/indicator";
import type { ChipTone } from "../components/primitives";
import { outside } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { conversationWorking, openWords, sessionWorking, type ProjectOpen } from "../lib/project-summary";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** How many of a tool's sessions run now, outside ones too (AJ-E1529). */
function toolRunning(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): number {
  return (
    sessions.filter((s) => s.provider === toolId && isActiveStatus(s.status)).length +
    outside(conversations).filter((c) => c.tool === toolId && c.live).length
  );
}

/** A tool's sessions now, as a project's are counted: open (AI
 * Juggler's and other apps'), and working (one asking you or stuck is
 * open, not working; the 0.4 review). */
export function toolOpen(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): ProjectOpen {
  const mine = sessions.filter((s) => s.provider === toolId && isActiveStatus(s.status));
  const theirs = outside(conversations).filter((c) => c.tool === toolId && c.live);
  return { open: toolRunning(toolId, sessions, conversations), working: mine.filter(sessionWorking).length + theirs.filter(conversationWorking).length, apps: 0 };
}

/** A tool's facts in the project's words: "2 working · 5 open". */
export function toolFacts(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): string {
  return openWords(toolOpen(toolId, sessions, conversations));
}


/** A session state as a chip: its tone, and the rail's words for it. */
export function statusTone(status: string): { tone: ChipTone; label: string } {
  const chip = stateChip(status);
  const tone: ChipTone =
    chip.tone === "live"
      ? "live"
      : chip.tone === "success"
        ? "success"
        : chip.tone === "warning"
          ? "warning"
          : chip.tone === "danger"
            ? "danger"
            : chip.tone === "accent"
              ? "accent"
              : "neutral";
  // One source per fact: the state in the rail's own words.
  return { tone, label: STATE_WORDS[status] ?? chip.label };
}
