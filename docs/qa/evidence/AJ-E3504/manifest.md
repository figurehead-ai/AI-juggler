# Evidence manifest — AJ-E3504 Style families as tool data, and each area's kit

Families are tool data: `ToolSpec.floor` (`FloorFamilies`, a word for the
tool's CLI Apps and, where it differs, its desktop app's) in
`crates/juggler-adapter/src/spec.rs`, set by each adapter. The Floor reads
the word (`crates/juggler-floor/src/family.rs`, `Family::of`); the catalog
and kits are `crates/juggler-floor/src/kit.rs`; the colour table is
`crates/juggler-floor/src/colour.rs`. Gemini CLI and Cursor have no adapter
yet; their families (campus, lab) are in the catalog for when they come.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 `ToolSpec` names a style family (Claude Code workshop, the Claude app studio, Codex industrial, OpenCode maker garage, Gemini CLI campus, Cursor lab, neutral otherwise); the conformance kit checks every tool names one or knowingly takes neutral | `cargo test -p juggler-adapters --test adapter_conformance names_a_floor_family`; `cargo test -p juggler-app --test floor_families`; adapters: `claude_code.rs` (`desktop: "studio"`, CLI `"workshop"`), `codex.rs` (`"industrial"`), `opencode.rs` (`"maker-garage"`), `grok_build.rs` and `kimi_code.rs` (`"neutral"`, on purpose), `fake*.rs` (`"neutral"`) | every adapter names a word for CLI and desktop; the Floor knows each word | the conformance check runs for claude_code, grok_build, kimi_code, opencode_v1, opencode_v2, codex and fake, and fails any tool with no word for CLI or desktop Apps or a word that isn't lowercase letters and hyphens; `every_tool_names_a_family_the_floor_knows` fails a word the Floor maps to neutral unless the word is `neutral`, and asserts at least four named families. Gemini CLI and Cursor: no adapter exists to name campus and lab; the families are in `Family::ALL` | PASS | f32c5ae4e | 2026-11-06 | Claude (self) |
| R2 the family catalog holds each family's floors, walls, desk models, chairs, plants and pots, dividers, nooks, signs, light warmth and layout types | `kit.rs`: `Kit` fields `floor` (`FloorMat`), `wall` (`WallFinish`), `desk` (`DeskKind`), `chair` (`ChairKind`), `plants` (`Plant`), `pot` (`Pot`), `divider`, `nook`, `sign` (`SignKind`), `warmth`, `layout`; one builder per family (`workshop`, `studio`, `industrial`, `garage`, `campus`, `lab`, `neutral`); `family.rs` `Family::layouts()` | every listed part, per family | all eleven parts are fields of `Kit`, each family's builder picks from its own options (for example workshop: parquet or planks; brick, paint or slats; trestle, four-leg or pedestal desks; terracotta or basket pots; library or sofa nook; warmth 0.9); layout types per family from `Family::layouts()` | PASS | 835678880 | 2026-11-06 | Claude (self) |
| R3 a kit is built from the family and the App's saved choice; big things vary between kits, props per desk; the App's colour slot marks its sign, flag, rug, badges and logo ring | `cargo test -p juggler-floor kit::tests::a_choice_is_the_same_for_the_same_app_and_seed_and_new_after_recreate`; `kit.rs` `kit_for(family, choice, slot)` (`accent: mark_colour(slot)`, `Rng(choice.seed)`); `scene/area.rs`: `tex::sign_img(…, a.kit.accent, …)`, `flag()` (glow in `a.kit.accent`), `tex::rug_img(a.kit.rug, a.kit.accent, …)`, `tex::logo_img(…, a.kit.accent)`; `tex.rs` `logo_img` (ring discs in `accent`); `overlay.rs` (area labels and waiting flags take `kit.accent`) | built from family + choice; accent on all five marks | test asserts a choice is the same for the same App and seed, within the family's layouts, and changes across seeds (Re-create). The accent on sign, flag, rug, logo ring and the overlay's badges is code reading; no test asserts the colour on each mark | PASS | 835678880 | 2026-11-06 | Claude (self) |
| R4 the Rust copy of `MARK_COLOURS` tested against `ui/src/lib/mark-hues.ts` | `cargo test -p juggler-floor colour::tests::the_mark_colours_match_the_main_windows` | the two tables equal | the test reads `mark-hues.ts` at compile time (`include_str!`), parses each `{ l, c, h }` row of `export const MARK_COLOURS` and asserts the list equals `colour::MARK_COLOURS` | PASS | 3cdd47b1d | 2026-11-06 | Claude (self) |
| Gate: `just tool-literals` green | `just tool-literals` | green | no recorded run in the repo; by reading, the family words live only in the adapters' `ToolSpec` and the Floor matches words, not tool ids | UNPROVEN | f32c5ae4e | 2026-11-06 | Claude (self) |
| Gate: two Apps of one tool differ in at least two of floor, wall, layout and sign; two tools differ in family | `cargo test -p juggler-floor kit::tests::two_apps_of_one_family_differ_in_big_things_and_families_differ` | every pair of one family differs in ≥ 2 | the test builds 50 pairs per family and allows up to 30 of 50 pairs to differ in fewer than two; it does not assert the claim for every pair. Two families differ in family and floor (asserted) | UNPROVEN | 835678880 | 2026-11-06 | Claude (self) |
| Gate: the colour-table test | as R4 | passes | as R4 | PASS | 3cdd47b1d | 2026-11-06 | Claude (self) |
| Budget: building every kit ≤ 1 ms | `cargo test -p juggler-floor kit::tests::building_every_kit_is_quick` | ≤ 1 ms | the test builds 140 kits (20 Apps × 7 families) and asserts under 50 ms; no record of the ≤ 1 ms figure | UNPROVEN | 835678880 | 2026-11-06 | Claude (self) |
| Evidence: a kit table for the sample, matching the prototype's log | `Kit::variant()` gives the words | the table, matching | not made (the handover note lists it as left) | UNPROVEN | — | 2026-11-06 | Claude (self) |

## Binary artifacts

None. The look of each family is covered by AJ-E3513's gallery. Binary
evidence, when added, is content-addressed: `<claim>-<sha8>.png|log|json`;
a re-run adds a new file, cited artifacts are never overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
