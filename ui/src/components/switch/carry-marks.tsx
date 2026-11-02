/**
 * 0.3.5 — the switch's chips, told apart at a glance (the maintainer,
 * 2026-11-02: "too bland, too monotone"): each kind of thing has its icon
 * (the conversation a bubble, the rules a page, connected tools a link…),
 * and each disposition its tint: goes along in the success tint, recreated
 * in the accent tint, not there yet or left behind in the warning tint. The
 * words always say it too; colour is never the only signal.
 */
import type { JSX, SVGProps } from "react";
import { IconGitBranch, IconSubtasks } from "../icons";
import { IconPage, IconRename, IconSliders } from "../icons-actions";
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
  moves: { chip: "border-success/30 bg-success-soft", icon: "text-success" },
  recreated: { chip: "border-accent/30 bg-accent-soft", icon: "text-accent-hi" },
  missing: { chip: "border-warning/35 bg-warning-soft", icon: "text-warning" },
  stays: { chip: "border-warning/35 bg-warning-soft", icon: "text-warning" },
};

/** A carry item's icon, by its kind (a check for one without a kind). */
export function CarryIcon({ kind, className = "" }: { kind: string | undefined; className?: string }): JSX.Element {
  const Mark = KIND_ICONS[kind ?? ""] ?? IconCheck;
  return <Mark aria-hidden="true" width={14} height={14} className={"shrink-0 " + className} />;
}
