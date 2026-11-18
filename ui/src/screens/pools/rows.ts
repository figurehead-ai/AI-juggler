/**
 * 0.4.6 (D-126): the rows New pool reads a pool back in — its preview
 * beside every step and its Review — each by one short title and one
 * icon (the mock-ups'), so the two tables can't name a row two ways. The
 * canvas checklist's names are STEP_WORDS' `short`.
 */
import type { ControlIcon } from "../../components/controls";
import { IconCompare, IconKey } from "../../components/icons-actions";
import { IconLead, IconMessage } from "../../components/icons-canvas";
import { IconProfiles, IconProjects, IconSessions, IconShield } from "../../components/icons";

export type PoolRow = "where" | "who" | "lead" | "everyone" | "talk" | "own" | "sealed" | "tools";

export const ROW_WORDS: Record<PoolRow, { title: string; icon: ControlIcon }> = {
  where: { title: "Where", icon: IconProjects },
  who: { title: "Who's in", icon: IconProfiles },
  lead: { title: "Lead", icon: IconLead },
  everyone: { title: "Roles", icon: IconShield },
  talk: { title: "Messages", icon: IconMessage },
  own: { title: "On their own", icon: IconCompare },
  sealed: { title: "Kept apart", icon: IconKey },
  tools: { title: "Team tools", icon: IconSessions },
};
