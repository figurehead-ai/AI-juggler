# Evidence manifest — AJ-E1526

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1526 (tool and company marks
everywhere; the maintainer's request, 2026-10-21). Status:
**DONE-EVIDENCED**. 2026-10-21, reviewer claude-opus.

## What exists now

- **`ToolMark`** (`ui/src/components/ToolMark.tsx`): the one component
  for a tool's mark, or its company's (`of="vendor"`). The mark id comes
  from the tool's ToolSpec (`mark`, `vendor_mark`, AJ-E1525); there are
  no per-tool branches. It draws a registered mark only where its owner
  allows it, as monochrome path data in `currentColor`. Otherwise it
  draws AI Juggler's neutral letter mark: the name's initials, or its
  first two letters, in a rounded square. It is always `aria-hidden`,
  beside the visible name.
- **The registry** (`ui/src/assets/third-party-marks/`): `index.ts`
  holds `MARKS`, and `NOTICE.md` says the folder is outside the MIT
  licence. Everything is bundled and nothing is fetched at runtime.
- **Where it shows** (Req.4):
  - Add account cards, and first run's group headings;
  - account cards (Accounts);
  - the rail's rows at their fuller detail level, through `providerMark`,
    now catalog-driven, so its hand-drawn per-tool glyphs and the
    `PROVIDER_MARKS` table are gone;
  - the inspector's tool header and its tool chips (Overview,
    desktop sessions);
  - the context-tool chip in the top bar;
  - Usage's account cards;
  - This Mac's tool lines, through a new `tool` field on `ToolTreeInfo`.
    A session gets its account's tool; an observed tree gets the tool
    This Mac observes.
- **The rights record.** `docs/research/brand-marks.md` gives each mark's
  source, the drawing's licence, the owner's guidelines and the date
  checked. TRADEMARKS.md gains "Other companies' names and logos".

## Brand decision (Req.2)

simple-icons 16.35.0 (CC0-1.0) has `claude`, `claudecode` and
`anthropic`; it has no OpenAI mark. CC0 covers the drawing, not the
trademark:

- Anthropic's trademark guidelines allow the marks only as specifically
  permitted, in materials approved beforehand;
- OpenAI's brand page asks for permission for logo use.

So no third-party mark is bundled today. Claude Code and Codex show the
letter mark (CC, Co) until approval is given. Allowing a mark later is
data only: add its row in `brand-marks.md` and its path in `MARKS`,
then re-render.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 each ToolSpec names its mark, and the vendor mark where it differs | `claude_code_spec` (`claude`, `anthropic`), `codex_spec` (`openai`); `tests/tools_catalog.rs`; vitest `tool-mark`: every catalog mark id has a registry entry | PASS |
| Req.2 sources, licence and guidelines recorded; a forbidden or missing mark falls back | `docs/research/brand-marks.md`; vitest: a disallowed mark is never drawn, and an unknown tool gets the letter mark | PASS |
| Req.3 never altered; monochrome `currentColor` | vitest: an allowed mark is drawn from its path in `currentColor`, unchanged | PASS |
| Req.4 one `ToolMark` everywhere, ids from the catalog | vitest: every catalog tool, tool and vendor, resolves to its drawable mark or the letter mark; `SessionRow` (an unknown tool by its id with the letter mark); `this-mac` (a line carries its tool's mark; a line with no known tool carries none); tool-literal lint: `indicator.tsx` drops off the allowlist (149 sites left) | PASS |
| Req.5 own folder outside MIT; TRADEMARKS.md; bundled | `ui/src/assets/third-party-marks/NOTICE.md`; TRADEMARKS.md "Other companies' names and logos" | PASS |
| Marks are decorative beside the visible name | vitest: `aria-hidden` on every mark; the Add account card still names "Claude Code"; the a11y gate | PASS |
| Re-rendered baselines with notes | the visual gate `--update`; the baselines it changed are listed under "## Visual baselines" in the release PR | PASS |

## Walked by hand

The vite dev server with the preview fixture, at 800×600, dark:

- Home's This Mac lines carry CC beside "Claude app" and the session's
  name.
- Accounts' cards carry CC in their tool tile.
- A running session's header chip reads "CC Claude Code".
- The DOM check: every `[data-mark]` is `aria-hidden` and sits beside
  its name.

## Gates

just verify, cargo deny, vitest, Playwright (full), the a11y gate, and
the visual gate `--update` in Docker. Results are in the release PR.
