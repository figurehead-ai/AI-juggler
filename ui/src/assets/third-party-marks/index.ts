/**
 * AJ-E1526 — the third-party marks AI Juggler may draw, by mark id (the
 * id a ToolSpec names). See NOTICE.md: these are not under the MIT
 * licence, and a mark is drawn only where `allowed` says its owner's
 * guidelines or written permission allow it. Each entry mirrors a row of
 * `docs/research/brand-marks.md`.
 */
export interface MarkEntry {
  /** Whose mark it is. */
  owner: string;
  /** Drawn only when true (guidelines allow it, or permission is given). */
  allowed: boolean;
  /** Monochrome path data on a 24×24 grid, drawn in currentColor; absent
   * while the mark may not be drawn. */
  path?: string;
}

export const MARKS: Record<string, MarkEntry> = {
  // Anthropic's trademark guidelines: use only as permitted, in materials
  // Anthropic approves beforehand (marketing@anthropic.com).
  claude: { owner: "Anthropic", allowed: false },
  anthropic: { owner: "Anthropic", allowed: false },
  // OpenAI's brand page: logo use by permission (partnercomms@openai.com).
  openai: { owner: "OpenAI", allowed: false },
};
