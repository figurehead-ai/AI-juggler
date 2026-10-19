# Evidence manifest — AJ-E1106

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1106 (guardrail tests and
the public metrics document). 2026-10-18, reviewer claude-opus. Log:
[`guardrail-suite.txt`](guardrail-suite.txt) (the stats crate, the app's
stats unit tests, `stats_guardrails`, `stats_sync`).

## Every promise and the test that keeps it

| Promise | Test(s) |
|---|---|
| Default silence: no folder, zero requests | `stats_guardrails::without_a_choice_nothing_is_created_and_nothing_is_sent`; `juggler-stats` `tests::off_creates_no_folder_and_records_nothing`, `cycle::tests::off_does_nothing_and_creates_nothing`; app `usage_stats::tests::off_and_local_send_nothing`, `default_is_off_and_creates_nothing` |
| The overrides, each with its reason | `mode::tests::overrides_win_in_order_with_their_reason` (DO_NOT_TRACK beats all; `AI_JUGGLER_STATS=off`/`local`; CI; debug builds); app `overrides_show_their_reason`; Privacy states in `privacy-card.test.tsx` |
| No forbidden content in an assembled report (an Account label, a project under `$HOME`, a private model name, an error with a secret-shaped string, the home path, the user name) | `stats_guardrails::nothing_the_user_owns_reaches_the_stats_store_or_a_report` (driven through the real counting paths and the week close; checks every byte of the stats folder, store included); `error_classes_come_from_the_variant_never_the_text`; `the_interface_cannot_widen_what_is_counted`; `mappings_keep_only_schema_words` |
| No reads of vendor credential paths during stats code | `stats_guardrails::stats_code_names_no_vendor_credential_or_identity_file` (the stats crate and `usage_stats.rs` name no credential or identity file); the weekly CLI-version gauge runs only `--version` on the Provider binary |
| Schema parity and validation | `tests/parity.rs` (the table equals the schema; one typed variant per metric; parse round-trips and rejects everything else; the generated file is current); `report::validate` in the assembly tests |
| Consent windows | `weeks_without_full_consent_or_in_local_are_skipped`; `ci_debug_and_unanswered_consent_never_send`; `a_week_the_sync_only_partly_covered_goes_the_anonymous_way` |
| Sampling | `sampling_hits_its_rate_within_two_percent`; `the_sample_rate_applied_is_in_the_report` |
| Backoff and expiry | `network_errors_back_off_and_give_up_after_six_attempts`; `retry_after_is_honoured_and_capped`; `expiry_ends_retries`; `a_week_assembled_too_late_expires` |
| Off means gone | `delete_all_removes_everything`; app `choosing_off_deletes_the_folder_and_says_so`; Privacy `choosing Off deletes and says so` |
| The cargo feature | `scripts/stats_feature_check.sh` (`just stats-events`, CI): in the default build, absent without the feature; clippy runs both ways |
| The 8 KB worst case | `the_worst_case_week_stays_under_8_kb` |
| Synced never doubles | `a_fully_synced_week_goes_only_to_the_account_unsampled`; `stats_sync::a_synced_week_goes_to_the_account_with_the_token_and_never_anonymously` |
| The puzzle vectors | `pow::tests::the_solver_reproduces_the_checked_vectors` (checked with the service's own verifier) |
| The user agent on every upload | app `a_debug_build_uploads_to_the_local_endpoint_with_the_contract_headers` (config GET and report POST, read off the wire) |

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Every promise in Req.1 has a test, and all are green in `just verify` | Req.1 | The table above; 79 tests in `guardrail-suite.txt`, all passing | PASS | 2026-10-18 | claude-opus |
| `docs/usage-stats.md` generated from the schema with the modes, the never-collected list, every way to turn it off, server retention and a sample report; CI fails on drift | Req.2 | `scripts/gen_usage_stats_doc.py` (AJ-E1101); `just stats-doc` in verify and a CI step; the document's sections: the three modes, how a report works (retention), never collected, turning it off, the optional sync, crash reports, a sample report, every metric | PASS | 2026-10-18 | claude-opus |
| The guardrail suite ≤ 30 s | perf budget | ≈ 1 s of test time (0.20 + 0.08 + 0.65 + 0.10 + 0.05 s); 7.1 s with cargo | PASS | 2026-10-18 | claude-opus |
