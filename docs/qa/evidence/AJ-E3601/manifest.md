# Evidence manifest — AJ-E3601

Task: AJ-E3601 (the calm foundation, E36, D-117). Status: **IN-PROGRESS**:
R1–R7 and R9 are built and tested; R8's baseline glance run and the
Increase Contrast captures are still to do.

Built in fa5b883d8 (tokens, App tone, Increase Contrast, Reduce Motion),
151b11d77 (the classic flag), 975fb8ec0 (the budget gate, contrast pairs,
the colourfulness metric, the glance sheet), 21633a541 (the selected
look) and 379acac83 (`attentionLevel()`); the §(a) table's last rows and
the speed budget were added with this manifest (the commit that adds this
file).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 `attentionLevel()` in `ui/src/lib/attention.ts`, the one source, every row of proposal §(a) placed, the backend's mirror held to the same table | vitest `ui/src/lib/attention-level.test.ts` (one case per row of `ui/src/lib/fixtures/attention-levels.json`, now with the L3 crashed, failed switch and degraded readings, and the L1 update, renewal, kept alive, keep alive paused, not measured and uncommitted rows); cargo `juggler-app` `features::notifications::centre::tests::attention_levels_match_the_windows_table` | every case equal on both sides | 65 vitest cases pass; the Rust test passes | PASS | this file's commit | 2026-11-18 | Claude Opus (self) |
| R1 speed: 0.05 ms an item | vitest `ui/src/lib/attention-level.test.ts` "places 10,000 items in 500 ms or less" | ≤ 500 ms for 10,000 | pass (well under) | PASS | this file's commit | 2026-11-18 | Claude Opus (self) |
| R1 no per-tool branch | `rg -n "claude\|codex\|provider" ui/src/lib/attention.ts` | no tool named | none | PASS | 379acac83 | 2026-11-18 | Claude Opus (self) |
| R2 the tokens: section, fill, divider, count and alarm count with their inks, the four level tokens, the tone; the dark status ladder retuned; the washes and logo glow gone | vitest `ui/src/lib/attention-tokens.test.ts` (every text token AA on the five surfaces in both themes, the badges' ink, danger on its tint, the dark APCA floors) | AA everywhere; warning ≥ Lc 60, danger ≥ Lc 52 in dark | pass | PASS | fa5b883d8 | 2026-11-18 | Claude Opus (self) |
| R2 naming note | `ui/src/styles.css` line 64: the section colour is `--color-section-label` (a `--color-section` token turned 101 `text-section` titles that colour) | named as agreed with the 0.4 line otherwise | the one rename is documented in the stylesheet | PASS | fa5b883d8 | 2026-11-18 | Claude Opus (self) |
| R3 one App tone treatment, `.app-tone` lit by `.is-lit` (or a `.tone-host` hovered, focused or `data-lit`), on the image `AppIcon` renders and on the `ProfileBadge` dot, nothing baked into assets | `ui/src/styles.css` `.app-tone`; `appTone()` passed as `className` to `AppIcon` and `ProfileBadge` in HomeApps, PlaceHeader, PlaceRows, RailFolded, RailProjectRows, WorkflowRail, OutsideConversations, HomeFeedCard | muted at rest; full at L2/L3, selected or current, hover or focus | as built | PASS | fa5b883d8 | 2026-11-18 | Claude Opus (self) |
| R4 Increase Contrast: divider to hairline, hairline to faint, ghost buttons bordered, filled controls outlined, App icons at `saturate(.5)` full opacity; Reduce Motion: no entry fade, no tone transition | `ui/src/styles.css` `@media (prefers-contrast: more)` (two blocks) and `@media (prefers-reduced-motion: reduce)` (`.feed-fresh`, `.app-tone`) | as R4 | as built | PASS | fa5b883d8 | 2026-11-18 | Claude Opus (self) |
| R4 captures with Increase Contrast on | the gallery's Home and rail with `forcedColors`/`contrast: more` emulation, both themes | look as R4 says | not captured | PENDING | — | — | — |
| R5 the hidden classic flag, 0.5 betas only, a test that fails once a 0.5 Stable sign-off exists while the flag remains | vitest `ui/src/lib/attention-classic.test.ts`; `ui/src/bundle/attention-classic-beta-only.test.ts` | read in a try/catch; no Settings, no baselines; the guard test present | pass | PASS | 151b11d77 | 2026-11-18 | Claude Opus (self) |
| R6 the attention-budget gate: allStatuses, typical and flood, both themes, header, rail and Home, items (i)–(vii) | `cd ui && pnpm exec playwright test tests/visual/attention-budget.spec.ts` | all green | see the AJ-E3603 manifest's run | PASS | 975fb8ec0 | 2026-11-18 | Claude Opus (self) |
| R7 the colourfulness metric | `python3 scripts/qa/colourfulness.py --selftest`; numbers in 975fb8ec0 (typical Home, dark, rail: M 25.3 → 11.2, loud 2.41 % → 0.48 %) | selftest OK; rail loud ≤ 1.5 % | selftest OK | PASS | 975fb8ec0 | 2026-11-18 | Claude Opus (self) |
| R8 the glance test and colour-vision walk sheet | `docs/qa/0.5-walks/calm-glance.md` | a reusable sheet | written | PASS | 975fb8ec0 | 2026-11-18 | Claude Opus (self) |
| R8 the sheet's baseline run on today's build | Sonnet persona glance per the sheet, verdicts checked by Opus | 4 of 5 right at 5 s | not run | PENDING | — | — | — |
| R9 contrast gate: the count badges, the section colour, the dark APCA floors; the mark hues re-run against the new status hues | `ui/tests/visual/contrast.spec.ts` (`TEXT` includes `section`; `BADGES`); vitest `ui/src/lib/rail-model.test.ts` (reads styles.css's status colours; ΔE ≥ 0.12 and the hue gaps) | all pairs pass | rail-model passes; contrast.spec in the a11y run | PASS | 975fb8ec0 | 2026-11-18 | Claude Opus (self) |

## Design reference (not evidence)

The round-2 mock the maintainer approved on 2026-11-14 (D-117), built by
hand from generic fixture names, no real accounts. Each image shows
before, round 2, and round 2 with a crashed session (L3). Opened and
checked by eye on 2026-11-14 before it was committed.

- `mock-r2-dark.png`: dark theme.
- `mock-r2-light.png`: light theme.

The proposal, research and audit behind them:
`docs/research/calm-attention/`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
