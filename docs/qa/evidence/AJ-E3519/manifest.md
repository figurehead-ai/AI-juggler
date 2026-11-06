# Evidence manifest — AJ-E3519 The View menu, top right

Built in db8cc4e7e (the View button in the overlay, the native popup menu
with check marks, `floor.show/arrange/labels/light/detail` keys,
`layout::arranged`, the Settings route). The walk of every item with
timings is `walk.md` here (AI Juggler Dev).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a View button top right opens a native menu: Show, Arrange, Labels, Light, Look, Settings › The Floor… | `cargo test -p juggler-floor driver::tests::lift_buttons_page_keys_and_the_view_button` (click → `Effect::ViewMenu` under the button); `VIEW` table in `features/floor/window.rs`; every capture in AJ-E3515…E3520 shows "VIEW ▾" top right | the six sections with the specified words | test passes; the table holds Show (All Apps, Active Apps only), Arrange (Kept places, Active Apps packed together), Labels (Smart, Attention only, All names), Light (Follow my clock, Always day, Always night), Look (Pixel art, Full resolution) and Settings › The Floor…; the menu itself: walk.md | PASS | db8cc4e7e | 2026-11-06 | Claude (self) |
| R2 each choice applies at once, is remembered as a `floor.*` key, shows its check | `cargo test -p juggler-app --lib features::floor::window::tests::every_view_menu_item_sets_a_setting_the_floor_has features::floor::store::tests::the_view_menu_s_choices_read_their_words_with_defaults` | every item maps to a key the Floor reads; defaults All, Kept, Smart, Follow my clock, Pixel art | tests pass: every menu id resolves to its (key, word) and the store reads each word back with its default; unknown ids ignored | PASS | db8cc4e7e | 2026-11-06 | Claude (self) |
| R3 Packed is a view only; Kept places brings every area back exactly | `cargo test -p juggler-floor layout_tests::packed_is_a_view_only_and_kept_places_come_back_exactly` | the office unchanged by Packed; Kept restores the plan | passes ("Packed never writes the office"; the kept plan equal after a round trip) | PASS | db8cc4e7e | 2026-11-06 | Claude (self) |
| Gate: store tests for the keys | `cargo test -p juggler-app --lib features::floor::store` | all pass | pass | PASS | db8cc4e7e | 2026-11-06 | Claude (self) |
| Gate: Packed never writes the office | layout test above | passes | passes | PASS | db8cc4e7e | 2026-11-06 | Claude (self) |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (none)
