# Evidence manifest — AJ-E1109

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1109 (personal stats
sync). 2026-10-18, reviewer claude-opus. Live run:
[`live-local-server.txt`](live-local-server.txt). Capture:
`_gallery/62-privacy/synced-*`.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Offered only while sharing on a linked Mac whose account includes it: "Show my stats in my Figurehead dashboard", unticked by default, "Synced to …" when on | Req.1 | `UsageSyncInfo.offered` = sharing ∧ linked ∧ entitlement; the choice is the account's `usageSync`, set only from Settings › Privacy (Account & Cloud shows it and points there). `privacy-card.test.tsx` › AJ-E1109 (unticked, then "Synced to you@example.com"; not offered otherwise); `usage_stats_set_sync_impl` refuses unless sharing | PASS | 2026-10-18 | claude-opus |
| A week is synced only if the sync covered all of it; then to the account endpoint with the device token, unsampled, no puzzle, and never also anonymously | Req.2 | `cycle::tests::a_fully_synced_week_goes_only_to_the_account_unsampled` (one request, the token, no config fetch, `sample_rate` 1, no `pow`); `a_week_the_sync_only_partly_covered_goes_the_anonymous_way` (anonymous, no token, nothing to the account). App: `stats_sync::a_synced_week_goes_to_the_account_with_the_token_and_never_anonymously` | PASS | 2026-10-18 | claude-opus |
| `401` unticks, skips the report (`unlinked`), says "This Mac is no longer linked, so stats sync is off.", and never falls back to anonymous | Req.3 | `cycle::tests::a_401_from_the_sync_unlinks_and_never_falls_back_to_anonymous`; app `stats_sync::a_401_unlinks_keeps_the_report_and_sends_nothing_anonymously` (unlinked locally like any other 401: every cloud toggle off; the notice; usage stats stay as chosen) | PASS | 2026-10-18 | claude-opus |
| A run against the website's local dev server | evidence | `live-local-server.txt`: linked, a real assembled report → `202`; the same report again → `200 {"duplicate":true}`; no token → `401`; after unlink → `401` | PASS | 2026-10-18 | claude-opus |

Never both is structural: each pending report goes down exactly one
path, chosen per week from `sync_since` (set when the sync turns on,
cleared when it turns off), and a synced report is filed as sent with a
`synced` mark.
