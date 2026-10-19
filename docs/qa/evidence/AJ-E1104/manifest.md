# Evidence manifest — AJ-E1104

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1104 (weekly assembly,
sampling, config, upload and backoff). 2026-10-18, reviewer claude-opus.
Logs: [`cargo-test-juggler-stats.log`](cargo-test-juggler-stats.log)
(fake-clock tests), [`live-local-server.log`](live-local-server.log)
(the service's local development server).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Each closed ISO week (UTC) is assembled once: gauges snapshotted, rounding, caps clamped (counted in `stats.truncated`), validated, `pending/` with an upload time 0–144 h after close; `partial_consent`, `local`, `invalid`, `expired` (> 13 days) go to `skipped/` with the reason | Req.1 | `cycle::tests::a_closed_week_is_assembled_once_into_pending` (tokens 123,456 → 120,000; histogram `[0,0,3,0]`; rows deleted, never assembled twice); `weeks_without_full_consent_or_in_local_are_skipped`; `a_week_assembled_too_late_expires`; `the_worst_case_week_stays_under_8_kb` (every value of every dimension at its cap: clamped, 464 truncations counted, valid). Consent per week: sharing must have started before the week did; any change of the effective mode is noted at once (`note_mode`), so even a short gap makes the week partial | PASS | 2026-10-18 | claude-opus |
| Sampling `sha256(salt ‖ week)` against `sample_rate`; salt never sent; applied rate recorded | Req.2 | `sampling_hits_its_rate_within_two_percent`: 10,000 salts → 0.1008 at 0.1, 0.4878 at 0.5; stable within a week, varies across weeks. `the_sample_rate_applied_is_in_the_report`. The report type has no salt field (`deny_unknown_fields`) | PASS | 2026-10-18 | claude-opus |
| Config fetched at most once a day when a report is due, reused up to 7 days, obeyed (`enabled`, version range, schemas, `pow_bits`); the upload URL compiled in | Req.3 | `no_usable_config_means_no_upload_yet` (no copy → no upload; a stale copy is reused when the refresh fails; no second fetch within a day); `the_config_can_switch_off_narrow_and_sample` (`server_disabled`, `version`, `schema`, `not_sampled`, zero POSTs). `Endpoints::for_build` takes no config input | PASS | 2026-10-18 | claude-opus |
| One report per tick, oldest first, gzip JSON, `Content-Type: application/json`, `User-Agent: AIJuggler/<version>`, no cookies, 10 s timeout; 200/202 → `sent/`; 400/404/413 → `skipped/`; 429/503 → `Retry-After` (cap 72 h); network/5xx → 1, 3, 9, 24, 48 h ±20 %; give up after 6 attempts or at expiry | Req.4 | `a_due_report_uploads_once_gzipped_and_moves_to_sent` (the gzip body decodes to exactly the kept bytes; `upload.log` line); `several_weeks_go_oldest_first_one_per_tick`; `rejected_reports_are_not_retried`; `retry_after_is_honoured_and_capped`; `network_errors_back_off_and_give_up_after_six_attempts`; `expiry_ends_retries`. App: `a_debug_build_uploads_to_the_local_endpoint_with_the_contract_headers` (a local HTTP server sees `user-agent: aijuggler/0.0.1`, `content-type: application/json`, `content-encoding: gzip`, no cookie header; ureq is built without its cookie feature; 10 s via `UreqTransport::with_timeout`) | PASS | 2026-10-18 | claude-opus |
| Release builds ignore `AI_JUGGLER_STATS_ENDPOINT`; debug builds upload only there | Req.5 | `release_builds_ignore_the_endpoint_variable`; `ci_debug_and_unanswered_consent_never_send` (CI, a debug build without the endpoint, and an unanswered consent version: zero requests, report kept in `pending/`) | PASS | 2026-10-18 | claude-opus |
| Off and local make zero stats requests | spec §9 | `off_does_nothing_and_creates_nothing` (no folder, no requests); app `off_and_local_send_nothing` (recording transport: 0) | PASS | 2026-10-18 | claude-opus |
| Assembly < 50 ms; typical < 4 KB; ≤ 8 KB hard (`ui.*`, `perf.*`, then the rest) | perf budget | `assembly: 21 µs (best of 5)`; the live report is 400 bytes; worst case 8,192 bytes with no `ui.*` entries left | PASS | 2026-10-18 | claude-opus |
| A run against the local stats endpoint (debug build) | evidence | `live-local-server.log`: assembled → pending → uploaded → `202` → `sent/`; the same report again → `200 {"duplicate":true}` | PASS | 2026-10-18 | claude-opus |

Notes:

- Production's config has `"enabled": false` until launch, so a sharing
  user's reports are filed under `skipped/` as `server_disabled`. A direct
  upload would get `503` with `Retry-After`, which is handled too.
- This workspace is version 0.0.1, below the service's minimum (0.1.0):
  its reports are skipped as `version`. The live run presented 0.1.0.
- The tick runs at launch and then hourly on its own lowest-priority
  thread; in `off` it returns at once and creates nothing. Environment
  gauges and emitting sites are AJ-E1103.
