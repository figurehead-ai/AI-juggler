/**
 * ADR-0020 — a series' colour comes from what it is, never its rank: a
 * profile wears its badge's swatch (`accountColor`, the same on every
 * page), a project its mark's, a tool its place in the catalog, a model
 * its family. "Other" is a neutral grey, so it never reads as an entity.
 */
import { accountColor, markOf, paletteIndex, swatchColor } from "../../lib/palette";
import { tools } from "../../lib/tools";

/** The folded series' key (the backend's `fold::OTHER`). */
export const OTHER_KEY = "__other";

/** What a series stands for. */
export type GroupBy = "profile" | "tool" | "model" | "project";

/** Model families by name, each on a fixed swatch (palette.ts order). */
const FAMILIES: [RegExp, number][] = [
  [/opus/i, 4],
  [/sonnet/i, 3],
  [/haiku/i, 5],
  [/gpt|codex|\bo\d/i, 6],
  [/gemini/i, 1],
];

/** The colour of one series. */
export function seriesColor(groupBy: GroupBy, key: string): string {
  if (key === OTHER_KEY || key === "") return "var(--color-faint)";
  switch (groupBy) {
    case "profile":
      return accountColor(key);
    case "project": {
      const mark = markOf(key);
      return mark ? swatchColor(mark.swatch) : swatchColor(paletteIndex(`project:${key}`));
    }
    case "tool": {
      const at = tools().findIndex((tool) => tool.id === key);
      return swatchColor(at >= 0 ? at * 2 : paletteIndex(`tool:${key}`));
    }
    case "model": {
      const family = FAMILIES.find(([pattern]) => pattern.test(key));
      return swatchColor(family ? family[1] : paletteIndex(`model:${key}`));
    }
  }
}

/** A model's name as people say it: "claude-opus-4-5-20251101" → "Opus 4.5". */
export function modelName(id: string): string {
  if (!id) return "Model not recorded";
  const claude = /^claude-(opus|sonnet|haiku)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/i.exec(id);
  if (claude) return `${claude[1]![0]!.toUpperCase()}${claude[1]!.slice(1)} ${claude[2]}${claude[3] ? `.${claude[3]}` : ""}`;
  const legacy = /^claude-(\d+)(?:-(\d))?-(opus|sonnet|haiku)/i.exec(id);
  if (legacy) return `${legacy[3]![0]!.toUpperCase()}${legacy[3]!.slice(1)} ${legacy[1]}${legacy[2] ? `.${legacy[2]}` : ""}`;
  if (/^gpt-/i.test(id)) return id.replace(/^gpt-/i, "GPT-").replace(/-codex/i, " Codex");
  return id[0]!.toUpperCase() + id.slice(1);
}
