# Evidence manifest — AJ-E105 (Launch settings demotion)

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1-2 rename + alias, values unchanged | `cargo test -p juggler-core --test storage_roundtrip` (incl. launch_recipe_alias test) | values equal | 17 green; Profile→LaunchRecipe pure alias; table/JSON untouched | PASS | 2026-10-13 | zcode-primary |
| Req.3 UI: Start on [Account] + collapsed Launch settings | Accounts screen (gallery re-capture) | primary visible, settings collapsed | "Start on <label>" primary (disabled w/ reason when signed out), "Edit launch settings ▸" collapsed by default, helper copy present | PASS | 2026-10-13 | zcode-primary |
| Req.4 no new window.alert | grep window.alert | 0 added | 3 pre-existing (login/handover flows, unchanged); launch sheet uses none | PASS | 2026-10-13 | zcode-primary |
| Req.5 zero-named default | account create/import path | auto "Main" default overlay | create_account/import auto-create default recipe, no naming prompt | PASS | 2026-10-13 | zcode-primary |
| Visual: launch sheet states 1280×800 + 390×844 | gallery (Accounts screen cells) | collapsed/expanded captured; dirty/error listed | collapsed/expanded in gallery; dirty/error inline states DEFERRED → AJ-E306 (full launch-sheet screen 10 owns the edit-in-sheet flow) | DEFERRED → AJ-E306 | 2026-10-13 | zcode-primary |
| Full gates | just verify + pnpm build + visual:capture | green | 84 rust tests; ui green; 41 captures | PASS | 2026-10-13 | zcode-primary |
