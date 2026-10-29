/**
 * AJ-E1520 (0.3) — the Subagents views' words: a status as an icon and a
 * word (never colour alone), a type and a model in plain words, times
 * and token counts in a few characters, and the tree's lines.
 */
import type { ComponentType, SVGProps } from "react";
import type { PillTone } from "../../components/Pill";
import { IconStateFailed, IconStateFinished, IconStateRunning, IconStateStopped, IconStateUnknown } from "../../components/icons";
import { formatDuration } from "../../components/indicator";
import { compactCount } from "../../lib/compact";
import type { SubagentInfo } from "../../lib/ipc/subagents";

type Icon = ComponentType<SVGProps<SVGSVGElement>>;

export const STATUS: Record<string, { word: string; tone: PillTone; icon: Icon; hint: string }> = {
  running: { word: "Running", tone: "live", icon: IconStateRunning, hint: "Still working: nothing in its records says it ended." },
  done: { word: "Done", tone: "success", icon: IconStateFinished, hint: "It finished." },
  failed: { word: "Failed", tone: "danger", icon: IconStateFailed, hint: "It ended with an error." },
  stopped: { word: "Stopped", tone: "warning", icon: IconStateStopped, hint: "It was stopped before it finished." },
  unknown: { word: "Unknown", tone: "neutral", icon: IconStateUnknown, hint: "Its records don't say how it ended (its session has ended)." },
};

export function statusOf(word: string): (typeof STATUS)[string] {
  return STATUS[word] ?? STATUS.unknown!;
}

/** A type in plain words: `general-purpose` → "General purpose". */
export function kindWords(kind: string | null): string | null {
  if (!kind) return null;
  const words = kind.replace(/[-_]+/g, " ").trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** A model in a word or two: `claude-opus-4-1-20250805` → "Opus 4.1",
 * `gpt-5-codex` → "GPT-5 Codex"; anything else as it is. */
export function modelWords(model: string | null): string | null {
  if (!model) return null;
  const claude = /^claude-(opus|sonnet|haiku)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/.exec(model);
  if (claude) {
    const family = claude[1]!.charAt(0).toUpperCase() + claude[1]!.slice(1);
    return claude[3] ? `${family} ${claude[2]}.${claude[3]}` : `${family} ${claude[2]}`;
  }
  const gpt = /^gpt-([\d.]+)(?:-(.+))?$/.exec(model);
  if (gpt) return gpt[2] ? `GPT-${gpt[1]} ${gpt[2].charAt(0).toUpperCase()}${gpt[2].slice(1)}` : `GPT-${gpt[1]}`;
  return model;
}

/** A count in a few characters: "412k", "1.2M", "940". */
export function tokenWords(n: number | null): string {
  if (n === null) return "—";
  return compactCount(n, (value, suffix) => (suffix === "k" && value >= 100 ? 0 : 1));
}

/** How long it ran (or has run): "4m", "32s". */
export function timeWords(agent: SubagentInfo): string {
  return agent.duration_ms === null ? "—" : formatDuration(agent.duration_ms);
}

/** What a row is called: its task, else its type. */
export function titleOf(agent: Pick<SubagentInfo, "task" | "kind">): string {
  return agent.task ?? (agent.kind ? `A ${kindWords(agent.kind)?.toLowerCase()} subagent` : "A subagent");
}

/** The tree's lines before a row: `│ ├ `. */
export function treeLines(guides: readonly boolean[], last: boolean): string {
  return guides.map((down) => (down ? "│ " : "  ")).join("") + (last ? "└ " : "├ ");
}

/** A share, 0–100, as words: "41%", "0.6%", "<0.1%". */
export function shareWords(pct: number): string {
  if (pct > 0 && pct < 0.1) return "<0.1%";
  return pct < 10 ? `${Math.round(pct * 10) / 10}%` : `${Math.round(pct)}%`;
}
