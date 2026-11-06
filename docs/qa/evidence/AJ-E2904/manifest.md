# Evidence manifest — AJ-E2904

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2904,
notifications tuned to you. Built in 0.4. Checked on 2026-11-06
(release/0.4.1). vitest `when-to-tell.test.tsx`, `settings-revert.test.tsx`
and `system-notifications.test.tsx` pass (2026-11-06); the Rust tests
named below run in the release preflight. Status: **DONE, 3 gaps open**
(agent self-check, 2026-11-06; R1's UI part and R4 closed the same day;
what's left is the backend's: R1's count, R2, R3).

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | `crates/juggler-app/src/features/notifications/engine.rs` `a_wait_is_told_once_it_has_lasted_the_users_threshold` asserts nothing at 4 minutes, one at 5, none again in the same spell, and a short wait never told. `crates/juggler-app/tests/system_notifications.rs` sets the stored `notify.wait_after_min` to 0 and gets the wait at once, so the setting reaches the gate. `ui/src/components/when-to-tell.test.tsx` asserts the default 5 and saving Right away. **Closed (UI, 2026-11-06):** `ui/src/components/when-to-tell.test.tsx` "R1: offers Right away, 2, 5, 10, 15 and 30 minutes, 5 by default, and says Needs you shows a wait at once". **Missing (Rust):** no test that Needs you, the bell and the count show a wait at once with a 5-minute setting (`needs::work_out` in the backend). |
| R2 | GAP | `crates/juggler-core/tests/alert_rearm.rs` `the_first_and_last_warnings_come_at_the_users_levels_once_each` asserts first and last at 80/95 and at 50/70, once each; the other `alert_rearm.rs` tests keep once per window and the time-to-empty warnings. `crates/juggler-app/tests/settings_policy.rs` `notification_prefs_default_to_the_anti_fatigue_hours_and_persist` refuses a last level below the first; `when-to-tell.test.tsx` asserts the last Picker offers only levels above 80. **Missing:** an estimated reading doesn't say "about": the quota notice's words (`features/quota/alert_words.rs`, `AlertQuotaInfo`) carry no confidence, though the pane promises it. Nothing limits the warnings to measured or computed windows (`alerts::step` fires on any confidence). No test covers either. |
| R3 | GAP | `system_notifications.rs` `send_test_reaches_the_system_whatever_holds_the_others_back` asserts one notification with every kind off, in the quiet hours and with AI Juggler in front, titled "… test" with "Notifications reach you.", and words starting "Sent. If nothing appeared, allow ". `when-to-tell.test.tsx` asserts the button sends one and shows "If nothing appeared". **Missing:** no test for the system's words from `juggler_platform::system::notification_settings_words` (macOS "System Settings › Notifications"). The live Send test on this Mac, with a capture checked by eye, is not done. |
| R4 | PASS | `NotificationSettings.tsx` saves the three new Pickers through `saveOptimistic`. `settings-revert.test.tsx` "puts the quiet hours back and says why" and "the helper shows the new value at once, and the old one after a failure" test that path for the quiet hours. **Closed (2026-11-06):** `ui/src/components/settings-revert.test.tsx` "AJ-E2904 R4: puts a wait's threshold back and says why" (Right away refused: back to 5, the alert says why in the backend's words) and "AJ-E2904 R4: puts a warning level back and says why" (first 80 → 70 and last 95 → 99 refused: each back, each said). |
| R5 | PASS | `settings_policy.rs` `notification_prefs_default_to_the_anti_fatigue_hours_and_persist` asserts a new store reads 5, 80 and 95 and that saved values persist. The generated `ui/src/types/generated/NotificationPrefsInfo.ts` has `wait_after_min`, `quota_first_pct` and `quota_last_pct`. |
| R6 | PASS | The pane's words, `ui/src/components/NotificationSettings.tsx` "When to tell me": "Tell me when a session waits longer than" with "Right away" or "n minutes", "Warn when an account has used … and again at", levels as "80 %", and "Once each per 5-hour or weekly window." No "threshold" or window code appears in the file. `when-to-tell.test.tsx` asserts the saved line says "right away". |

Gallery: `ui/tests/visual/release-04-screens-gallery.spec.ts` "When to
tell me" (family `99-when-to-tell`) shows the section closed. Not yet
captured: each Picker open, Send test's line, a reverted save.

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.
