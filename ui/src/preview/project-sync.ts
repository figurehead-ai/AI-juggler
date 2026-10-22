/**
 * AJ-E1507: example data for the project sync sheet, for the tests and
 * the gallery (set it as `window.__AIJ_PROJECT_SYNC__`). Plain data: no
 * import that touches the page, so a Playwright spec can read it too.
 */
import type { RulesDiffInfo } from "../types/generated/RulesDiffInfo";
import type { RulesSyncInfo } from "../types/generated/RulesSyncInfo";
import type { ServersSyncInfo } from "../types/generated/ServersSyncInfo";

/** What the preview and the tests show instead of the backend. */
export interface ProjectSyncSeam {
  rules?: RulesSyncInfo;
  /** Diffs by `"<file>|<how>"`. */
  diffs?: Record<string, RulesDiffInfo>;
  servers?: ServersSyncInfo;
}

/** An example for tests and the gallery (set it as the seam): CLAUDE.md
 * was changed last; AGENTS.md has drifted; Codex lacks Work's server. */
export const EXAMPLE_PROJECT_SYNC: ProjectSyncSeam = {
  rules: {
    master: "CLAUDE.md",
    in_sync: false,
    summary: "1 rules file differs",
    files: [
      {
        name: "CLAUDE.md",
        tools: ["Claude Code"],
        standing: "master",
        words: "The master",
        modified_ms: 1792663200000,
        can_be_master: true,
        can_apply: false,
        import_line: null,
        sha256: "sha-claude",
      },
      {
        name: "AGENTS.md",
        tools: ["Codex"],
        standing: "differs",
        words: "Differs from CLAUDE.md",
        modified_ms: 1792404000000,
        can_be_master: true,
        can_apply: true,
        import_line: null,
        sha256: "sha-agents",
      },
    ],
  },
  diffs: {
    "AGENTS.md|copy": {
      file: "AGENTS.md",
      master: "CLAUDE.md",
      how: "copy",
      added: 2,
      removed: 1,
      groups: [
        [
          { change: "kept", text: "# Shop", old_number: 1, new_number: 1 },
          { change: "kept", text: "", old_number: 2, new_number: 2 },
          { change: "removed", text: "Use tabs.", old_number: 3, new_number: null },
          { change: "added", text: "Use two spaces.", old_number: null, new_number: 3 },
          { change: "added", text: "Run `pnpm test` before you commit.", old_number: null, new_number: 4 },
          { change: "kept", text: "Keep changes small.", old_number: 4, new_number: 5 },
        ],
      ],
    },
  },
  servers: {
    accounts: [
      { account: "acc-work", label: "Work", tool: "Claude Code", servers: ["docs", "github"] },
      { account: "acc-codex", label: "Codex", tool: "Codex", servers: ["docs"] },
    ],
    copies: [
      {
        server: "github",
        from: "acc-work",
        from_label: "Work",
        to: "acc-codex",
        to_label: "Codex",
        offer: "offer-github",
        verdict: "translate",
        note: "It goes into config.toml. GITHUB_TOKEN stays behind: set it in your environment, secrets are never copied.",
      },
    ],
  },
};
