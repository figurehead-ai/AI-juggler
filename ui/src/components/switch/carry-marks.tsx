/**
 * 0.3.5 — the switch's chips, told apart at a glance (the maintainer,
 * 2026-11-02: "too bland, too monotone"): each kind of thing has its icon
 * (the conversation a bubble, the rules a page, connected tools a link…),
 * and each disposition its tint: goes along in the success tint, recreated
 * in the accent tint, not there yet in the warning tint, left behind
 * neutral (0.5.0: a fact, not a call to act). The words always say it too;
 * colour is never the only signal.
 */
import type { JSX, SVGProps } from "react";
import { IconGitBranch, IconSubtasks } from "../icons";
import { IconPage, IconRemove, IconRename, IconSliders } from "../icons-actions";
import { IconMessage } from "../icons-canvas";
import { IconCheck } from "../icons-controls";
import { IconConnect } from "../icons-menu";
import { IconCode, IconTasks } from "../icons-places";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

/** The backend's carry kinds (`switch_words::kind_name`), and this
 * project's notes (`memory`, the set-up check's). */
const KIND_ICONS: Record<string, Icon> = {
  chat: IconMessage,
  rules: IconPage,
  changes: IconGitBranch,
  mcp: IconConnect,
  skills: IconCode,
  model: IconSliders,
  subagents: IconSubtasks,
  memory: IconRename,
  tasks: IconTasks,
};

/** How a chip's thing fares: goes along, recreated, not there yet, left
 * behind. */
export type CarryTone = "moves" | "recreated" | "missing" | "stays";

/** The chip's tint and its icon's colour, per disposition. */
export const CARRY_TONES: Record<CarryTone, { chip: string; icon: string }> = {
  moves: { chip: "bg-success-soft", icon: "text-success" },
  recreated: { chip: "bg-accent-soft", icon: "text-accent-hi" },
  missing: { chip: "bg-warning-soft", icon: "text-warning" },
  // Left behind is a fact, not a call to act (COL-4): neutral.
  stays: { chip: "bg-surface-3", icon: "text-ink-2" },
};

/** A carry item's icon, by its kind; one without a kind by how it fares:
 * a check where it goes along, a minus where it stays behind (never the
 * same mark on both sides of the list). */
export function CarryIcon({ kind, tone = "moves", className = "" }: { kind: string | undefined; tone?: CarryTone; className?: string }): JSX.Element {
  const Mark = KIND_ICONS[kind ?? ""] ?? (tone === "stays" ? IconRemove : IconCheck);
  return <Mark aria-hidden="true" width={14} height={14} className={"shrink-0 " + className} />;
}
