# Evidence manifest — AJ-E2503

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2503, the
notification centre (§4.14). Built 2026-10-27. Checked requirement by
requirement on 2026-11-06 (release/0.4.1). Five requirements have gaps,
so the status stays DONE-PENDING-EVIDENCE.

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | **Tested:** a dialog named "Notifications" (`ui/src/components/notifications.test.tsx` "says \"You're all caught up.\" when nothing is there"); the bell opens it ("shows the model's one count, and opens the centre"); ⌥⌘N opens it (`app-keys.test.tsx` "D-50: ⌥⌘N opens the notifications", and `tests/visual/keyboard-flows.spec.ts` "launch keys"); Esc closes it ("Esc closes it"). **Missing:** no test that focus moves into the dialog, that Tab stays inside (`keepTabInside`), or that focus goes back to the bell on close. |
| R2 | GAP | **Tested:** Needs action mirrors Needs you, with the heading counting its rows (`notifications.test.tsx` "lists Needs action and Earlier…", "its heading counts the rows under it…"; the gallery `tests/visual/notification-centre-gallery.spec.ts` checks `Needs action · n` = the bell = Home); the ⋯ menu is Snooze 1 h, Snooze until tomorrow, Mute this project, Mute this kind, with an icon on every line ("each need's ⋯ offers snooze and mute…"); a snoozed need folds and can be woken ("a snoozed need says when it is back…"); the second actions (Remind me at <time> as amended, Snooze 1 h, Mark reviewed: "words: the digest, and the second actions the design gives"); the quota line "7% of its 5-hour window left · resets at … · 2 sessions use it" ("never says an internal id…"); primaries Open, Switch back…, Continue in another app… (`open-limit.test.tsx`, `credits-room.test.tsx`). **Missing:** no test of a row's unread dot or kind mark, of the fold's label "Snoozed: n", of an asking row's "waiting n min" line, or of the Review and Sign in… primaries. |
| R3 | GAP | **Tested:** Earlier lists what happened (`notifications.test.tsx` "lists Needs action and Earlier…": a restart row under Earlier); the backend pages it newest first by cursor (`centre::tests::earlier_pages_walk_thirty_days_newest_first`). **Missing:** no UI test of a row's "time · detail", its link (See why, What's new), 20 rows a page, or "Show earlier" loading the next page. |
| R4 | GAP | **Tested:** Mark all read clears every dot and leaves the count (`notifications.test.tsx` "Mark all read clears every dot and no count"); the digest's words (`digestLine`: "Today: 4 finished · 1 restarted · 1 update · Work Max resets …", "words: the digest…") and its counts (`centre::tests::the_digest_counts_today_and_names_the_next_reset`). **Missing:** no test that the centre shows the digest line, or that its "Notification settings" link opens Settings › Notifications. |
| R5 | PASS | `notifications.test.tsx` "says \"You're all caught up.\" when nothing is there" (`notices-empty`), and "in the quiet hours, says what is muted and what still counts": "Muted until 09:00 — Needs you still tracks 1." (and `mutedLine("09:00", 2)` reads "…tracks 2."). The backend's quiet-hours end: `centre::tests::the_quiet_hours_say_when_they_end`. Both states are in the gallery (centre-empty, centre-muted). |
| R6 | GAP | **Tested (backend):** mutes are listed paged with their labels (`centre::tests::mute_labels_are_the_apps_own_words_for_a_kind`) and unmute works (`notices_storage.rs` `read_all_dismiss_and_mutes`). **Missing:** no UI test of Settings › Notifications › Muted (`NoticeMutes`): the list, its paging, and Unmute removing a row. |

Run on 2026-11-06: `pnpm exec vitest run src/components/notifications.test.tsx
src/app-keys.test.tsx src/open-limit.test.tsx src/credits-room.test.tsx`
(passed); `cargo test -p juggler-app --lib features::notifications` (48
passed). The gallery (`96-notification-centre`) was not run here.
