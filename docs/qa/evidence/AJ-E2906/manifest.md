# Evidence manifest — AJ-E2906

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2906, first run
shows its confidence. Built in 0.4. Checked requirement by requirement on
2026-11-06 (release/0.4.1). Status: **DONE, 2 gaps open** (agent
self-check, 2026-11-06). Still open from the task's own evidence list: the
replica walk with a Guessed and a Likely setup.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `ui/src/screens/first-run-confidence.test.tsx` "the row's word is the weaker of its name and its sign-in" (Found, Likely, Guessed, Likely for four setups); "says where each came from, never an internal id" ("Claude Code's usual folder", "a folder named like a Claude Code setup", "set in your shell's settings", "named by cc-switch"); "each row says its word and where it came from…" renders the found step and reads each row's word and source line. |
| R2 | PASS | `first-run-confidence.test.tsx` "Details lists each field with its word and source, and the scan's notes": the toggle flips `aria-expanded`; Name, Sign-in and Last used each show value, word and source; the scan's note shows verbatim; the tick doesn't change. "each field has its word and source…" checks `fieldRows`, and that an unknown confidence never reads above Guessed. |
| R3 | PASS | **Backend rule per field:** `crates/juggler-adapters/tests/inference.rs` `every_field_names_its_rule` (usual folder, name word, folder name; sign-in present, empty, system keychain; newest conversation, no conversations), with the lib tests `purpose_rules_first_match_wins`, `login_rules_cover_all_four_states`, `activity_tiers_bound_at_7_and_90_days` (Rust: run in the release preflight). The wire type carries `purpose_rule`, `login_rule`, `activity_rule` and `set_by` (`ui/src/types/generated/InferenceInfo.ts`, `DiscoveredInfo.ts`; kept fresh by `scripts/ts_types_fresh.sh`). **Never an id on screen:** "each row says its word…" asserts none of `vendor-default`, `home-pattern`, `env-override`, `other-manager`, `usual-folder`, `sign-in-empty` appears. |
| R4 | GAP | Words, not scores, and the preselection rule are proven: `first-run-confidence.test.tsx` (words only) and `ui/src/screens/first-run.test.tsx` "preselects what's signed in or used in 30 days, never what's already in". Missing: a Guessed row doesn't say "You can rename it after". The 0.4 design review (53) moved it to one footer line, "Names and plans can be changed later.", and the test now asserts the row has no "rename"; the task text was not amended to match. |
| R5 | GAP | First run: `first-run-confidence.test.tsx` "Look again is always there and keeps the ticks on setups still found; a new one comes in as the scan suggests" (ticks kept by path, a new Guessed setup comes in unticked). Missing: no test shows Look again in Accounts › Add account. `FirstRun` in `mode="add"` draws the same found step, but with nothing new it jumps straight to the fresh step (`FirstRun.tsx`, the `nothingNew` effect), which has no Look again; the gallery `first-run-confidence-gallery.spec.ts` opens Add account but doesn't check the button. |
| R6 | PASS | `first-run.test.tsx` "D-46 the optional Figurehead account" ("ends first run with two equal choices, nothing chosen for the user") and the "D-25 first run — the flow" tests pass unchanged. |

Run 2026-11-06: `pnpm exec vitest run src/screens/first-run-confidence.test.tsx src/screens/first-run.test.tsx` — pass.
Gallery: `ui/tests/visual/first-run-confidence-gallery.spec.ts` (`99-first-run-confidence`: the found step with each word, Details open; both themes, 1280×800 and 1024×700, axe gate at 1280).
