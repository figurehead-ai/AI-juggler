# Brand marks (AJ-E1526)

Which tool and company marks AI Juggler may draw, where each comes from,
under what licence and guidelines, and when that was last checked.
`ui/src/assets/third-party-marks/index.ts` mirrors this table: a mark is
drawn only where its row says **allowed**. Everything else gets AI
Juggler's own neutral letter mark (the name's initials, or its first two
letters, in a rounded square). The rules:

- A mark is bundled with the app; nothing is fetched at runtime.
- It is drawn monochrome in `currentColor`, unaltered; brand colour only
  where the guidelines require it.
- It always sits beside the tool's visible name, never instead of it, and
  is `aria-hidden`.
- The marks folder is not under the MIT licence (its `NOTICE.md`; the
  repository's `TRADEMARKS.md`).

`simple-icons` (CC0-1.0 for its SVG data) is the preferred source where it
has the brand. CC0 covers the drawing, not the trademark: its own
disclaimer leaves the trademark rules to each owner, so each owner's
guidelines decide.

| Mark id | Owner | Used for | Source checked | Licence of the drawing | Owner's guidelines | Allowed? | Checked |
|---|---|---|---|---|---|---|---|
| `claude` | Anthropic | Claude Code (tool) | simple-icons 16.35.0 (`claude`, `claudecode`) | CC0-1.0 | [Trademark guidelines](https://www.anthropic.com/legal/trademark-guidelines): marks only as specifically permitted, in materials Anthropic approves beforehand (marketing@anthropic.com) | **No** — letter mark until approved | 2026-10-21 |
| `anthropic` | Anthropic | Claude Code (vendor) | simple-icons 16.35.0 (`anthropic`) | CC0-1.0 | as above | **No** — letter mark until approved | 2026-10-21 |
| `openai` | OpenAI | Codex (tool and vendor) | not in simple-icons 16.35.0 | — | [Brand guidelines](https://openai.com/brand/): logo use needs permission (partnercomms@openai.com) | **No** — letter mark until approved | 2026-10-21 |

Tools coming in E16 (Gemini CLI, OpenCode, …) get a row here when their
ToolSpec names a mark; simple-icons 16.35.0 has `googlegemini` and
`opencode`, so each owner's guidelines are what remain to check.

## To allow a mark

1. Get the owner's written permission, or a guideline that allows use in a
   third-party app, and record it in this table with the date.
2. Copy the monochrome path from the pinned source into `MARKS` with
   `allowed: true`; don't redraw or recolour it.
3. Re-render the baselines that show it, with notes.
