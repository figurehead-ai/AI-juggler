# Evidence manifest — AJ-E1107

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1107 (crash reports
behind their own setting). 2026-10-18, reviewer claude-opus. Captures:
`_gallery/64-crash` (the setting and the next-launch question, 1280×800,
light and dark). **Status: IN-PROGRESS.** The on-device half is done and
tested. Sending waits for the reporting service and a signed, notarized
build (maintainer steps).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Setting `never` (default) / `ask` / `always`; `DO_NOT_TRACK` forces `never` | Req.1 | `crash.rs` (per Mac; never in a backup). `the_setting_and_answers_move_reports_and_never_deletes_them`; `crash-reports.test.tsx` (Never by default; DO_NOT_TRACK forces Never and disables the choice). The `crash_reports.setting` gauge now reads it | PASS | 2026-10-18 | claude-opus |
| `ask` keeps the report locally and asks on next launch with a full read-only preview: Send / Don't send / Always send | Req.2 | A panic hook keeps the report under `crashes/asking/`. `CrashReportPrompt` shows the whole report and the three answers (Always send also changes the setting; Escape is Don't send). Tests: `crash-reports.test.tsx` ("the next launch shows the whole report and asks…"; nothing asked under Never or Always); capture `64-crash/ask-*` | PASS | 2026-10-18 | claude-opus |
| Scrubbing on the Mac before anything is kept: the home directory → `~`, no user name, no URLs or query strings, secret-shaped strings masked, no environment, host name or user field, ≤ 50 backtrace lines | Req.3 | `crash::scrub` / `report_of`: `a_seeded_crash_is_scrubbed_before_it_is_kept` (a home path, the user name, an `sk-ant-…` key, a URL with a token and a query string: none survive; `~/…` kept for readable paths; the report has no `server_name`, `user` or environment field). App-internal breadcrumbs: the app keeps none, so a report carries none | PASS | 2026-10-18 | claude-opus |
| On-device limits: one per crash signature a day, five a day in total | Req.4 | `the_limits_hold_one_per_signature_and_five_a_day` (a repeat signature refused; the sixth refused; a new day starts over; `never` keeps nothing). The signature hashes where it happened plus the failure's kind, never its message | PASS | 2026-10-18 | claude-opus |
| The reporting service's address compiled into release builds only; minidumps tested in a signed, notarized build before they are enabled | Req.5 | `crash::service_address()` is `None` in debug builds and otherwise reads `AI_JUGGLER_CRASH_ENDPOINT` at compile time; no address exists yet, so a report the user chooses to send stays queued on the Mac, and the setting says so ("This build can't send crash reports yet — N kept on this Mac until one can"). **Not done:** the service itself (the spec names Sentry; the project, its DSN and limits are the maintainer's to create), sending, and native minidumps, which need a signed, notarized build (AJ-E1010) | OPEN | 2026-10-18 | claude-opus |
| No cost when `never` | perf budget | The hook checks one atomic and returns; nothing is written or read | PASS | 2026-10-18 | claude-opus |
